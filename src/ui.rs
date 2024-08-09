use crate::filters::Status;
use crate::App;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem};
use ratatui::{
    prelude::Style,
    text::Line,
    widgets::{block::Title, Block, Paragraph},
    Frame,
};

pub fn ui(app: &App, f: &mut Frame) {
    let (console_area, tools_area) = calculate_layout(f.size());

    render_logs(f, console_area, app);
    render_tools(f, tools_area, app);
}

fn render_tools(frame: &mut Frame, area: Rect, app: &App) {
    let mut string_instructions = vec![" ".into()];

    for filter in app.filters.iter() {
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
    string_instructions.push(
        "q".to_uppercase()
            .to_string()
            .bold()
            .black()
            .bg(Color::White),
    );
    string_instructions.push(">".bg(Color::White).black());

    let block = Paragraph::new(Line::from(string_instructions).centered());

    frame.render_widget(block, area);
}

fn render_logs(frame: &mut Frame, area: Rect, app: &App) {
    let logs_cs2: Vec<ListItem> = app
        .logs
        .iter()
        .map(|log_cs2| {
            let s = get_style(log_cs2.status.clone());
            let content = vec![text::Line::from(vec![
                Span::styled(log_cs2.value.clone(), s),
                /*   Span::raw(log_cs2.value.clone()), */
            ])];
            ListItem::new(content)
        })
        .collect();

    let title = Title::from(vec![
        " ".into(),
        "P".bold().blue(),
        "L".bold().yellow(),
        "G".red().bold(),
        " Console".bold().light_red(),
        "".into(),
    ])
    .alignment(Alignment::Center);

    let logs = List::new(logs_cs2).block(Block::bordered().title(title));
    frame.render_widget(logs, area);
}

fn calculate_layout(area: Rect) -> (Rect, Rect) {
    let main_layout = Layout::vertical(vec![Constraint::Percentage(95), Constraint::Percentage(5)]);
    let [console_area, tools_area] = main_layout.areas(area);
    (console_area, tools_area)
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
