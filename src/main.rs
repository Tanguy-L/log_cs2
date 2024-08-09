use color_eyre::eyre::{Ok, WrapErr};
use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use serde_json;
use std::fs;
use std::time::{Duration, Instant};
mod filters;
use filters::Status;
use filters::{Filter, ListFilter};
mod errors;
mod tui;
mod ui;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct LogCS2 {
    value: String,
    status: filters::Status,
}

#[derive(Debug)]
pub struct App {
    filters: Vec<Filter>,
    exit: bool,
    logs: Vec<LogCS2>,
    last_check: Instant,
    file_path: String,
    file_filters_path: String,
    last_events: HashMap<PathBuf, Instant>,
    debounce_duration: Duration,
    scroll: u16,
}

impl Default for App {
    fn default() -> Self {
        Self {
            scroll: 0,
            filters: Vec::new(),
            exit: false,
            logs: vec![],
            last_check: Instant::now(),
            file_path: "../cs2server.log".to_string(),
            file_filters_path: "src/filters.json".to_string(),
            last_events: HashMap::new(),
            debounce_duration: Duration::from_millis(100),
        }
    }
}

use strip_ansi_escapes::strip;

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        self.init_filters();

        self.read_and_process_file();

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        watcher.watch(self.file_path.as_ref(), RecursiveMode::Recursive)?;

        let tick_rate = Duration::from_millis(100); // Adjust this value to change refresh rate
        let mut last_tick = Instant::now();

        while !self.exit {
            self.on_tick();

            self.check_file_changes(&rx);
            terminal.draw(|frame| ui::ui(&self, frame))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                self.handle_events().wrap_err("handle events failed")?;
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }

            self.last_check = Instant::now();
        }
        Ok(())
    }

    fn on_tick(&mut self) {
        self.scroll = (self.scroll + 1) % 10;
    }

    fn check_file_changes(&mut self, rx: &Receiver<notify::Result<notify::Event>>) {
        while let std::result::Result::Ok(result) = rx.try_recv() {
            match result {
                std::result::Result::Ok(event) => {
                    for path in event.paths {
                        let now = Instant::now();
                        if let Some(last_event_time) = self.last_events.get(&path) {
                            if now.duration_since(*last_event_time) < self.debounce_duration {
                                continue; // Skip this event as it's too soon after the last one
                            }
                        }
                        self.last_events.insert(path.clone(), now);
                        self.read_and_process_file();
                    }
                }
                Err(error) => self.logs.push(LogCS2 {
                    value: format!("ERROR: {}", error),
                    status: Status::Error,
                }),
            }
        }
    }

    fn read_and_process_file(&mut self) {
        let file_path = &self.file_path;
        self.logs.drain(..);

        let read_file = fs::read_to_string(file_path).expect("ERROR : CANT READ THE FILE");

        for line in read_file.lines() {
            for filter in &self.filters {
                let raw_line = String::from_utf8_lossy(&strip(line)).into_owned();

                match filter.match_regex(line) && filter.is_on == true {
                    true => self.logs.push(LogCS2 {
                        value: raw_line,
                        status: filter.get_status(),
                    }),
                    _ => (),
                }
            }
        }
    }

    fn init_filters(&mut self) {
        // Open the file in read-only mode with buffer.
        let file = fs::File::open(&self.file_filters_path).unwrap_or_else(|e| {
            let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("unknown"));
            panic!(
                "Failed to open filters file. Error: {}. Current directory: {}",
                e,
                current_dir.display()
            )
        });

        // Read the JSON contents of the file as an instance of `User`.
        let json: ListFilter = serde_json::from_reader(file).unwrap();

        for filter in json.filters {
            self.filters.push(filter);
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        let keys: Vec<char> = self
            .filters
            .iter()
            .map(|f| f.key_code.chars().next().unwrap())
            .collect();
        match key_event.code {
            KeyCode::Char(c) if keys.contains(&c) => {
                if let Some(filter) = self.filters.iter_mut().find(|x| x.key_code.starts_with(c)) {
                    filter.toggle();
                    self.logs.drain(..);
                }
                self.read_and_process_file();
            }
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    match App::default().run(&mut terminal) {
        _ok => {
            tui::restore()?;
        }
    };

    Ok(())
}
