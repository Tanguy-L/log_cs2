use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Rect},
    prelude::Style,
    symbols::border,
    text::Line,
    widgets::{
        block::{Position, Title},
        Block, Paragraph, Widget,
    },
    Frame,
};

use serde_json;
use std::fs;
use std::time::{Duration, Instant};

mod filters;
use filters::Status;
use filters::{Filter, ListFilter};
mod errors;
mod tui;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            exit: false,
            logs: vec![],
            last_check: Instant::now(),
            file_path: "../cs2server.log".to_string(),
            file_filters_path: "./filters.json".to_string(),
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
            self.check_file_changes(&rx);
            terminal.draw(|frame| self.render_frame(frame))?;

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

    fn check_file_changes(&mut self, rx: &Receiver<notify::Result<notify::Event>>) {
        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(event) => {
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
        let file = fs::File::open(&self.file_filters_path).unwrap();

        // Read the JSON contents of the file as an instance of `User`.
        let json: ListFilter = serde_json::from_reader(file).unwrap();

        for filter in json.filters {
            self.filters.push(filter);
        }
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
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

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(vec![
            " ".into(),
            "P".bold().blue(),
            "L".bold().yellow(),
            "G".red().bold(),
            " Console".bold().light_red(),
            "".into(),
        ]);
        let logs_lines: Vec<Line> = self
            .logs
            .iter()
            .map(|x| Line::styled(x.value.clone(), get_style(x.status.clone())))
            .collect();

        let mut string_instructions = vec![" ".into()];

        for filter in &self.filters {
            string_instructions.push(filter.name.clone().bold());
            string_instructions.push(" ".into());
            if filter.is_on {
                string_instructions.push("Y".bold().green());
            } else {
                string_instructions.push("N".bold().red());
            }
            string_instructions.push(" ".into());

            string_instructions.push("<".bg(Color::White).black());
            string_instructions.push(
                filter
                    .key_code
                    .clone()
                    .to_uppercase()
                    .to_string()
                    .bold()
                    .black()
                    .bg(Color::White),
            );
            string_instructions.push(">".bg(Color::White).black());
            string_instructions.push(" ".into());
        }
        string_instructions.push("Quit".bold());
        string_instructions.push(" ".into());

        string_instructions.push("<".bg(Color::White).black());
        string_instructions.push("Q".to_string().bold().black().bg(Color::White));
        string_instructions.push(">".bg(Color::White).black());
        string_instructions.push(" ".into());

        let instructions = Title::from(Line::from(string_instructions));
        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))
            .bold()
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .border_set(border::ROUNDED);

        Paragraph::new(logs_lines)
            .left_aligned()
            .block(block)
            .render(area, buf);
    }
}

fn get_style(status: Status) -> ratatui::prelude::Style {
    match status {
        Status::Infos => Style::new().blue(),
        Status::Warning => Style::new().yellow(),
        Status::Error => Style::new().red(),
        Status::Custom => Style::new().green(),
        Status::Custom2 => Style::new().light_magenta(),
        Status::Custom3 => Style::new().cyan(),
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
