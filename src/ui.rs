use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{
        Color, Modifier, Style
    },
    terminal::Frame,
    widgets::{Block, Padding, Paragraph}
};

use crate::app::App;


pub fn render_app(app: &App, frame: &mut Frame) {
    let error_len = match app.error.as_ref() {
        None => 0,
        Some(err) => err.lines().count().clamp(4, 64) as u16
    };
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), Constraint::Length(error_len), Constraint::Length(1)]
    );
    let &[filtered_content, errors, query_edit] = layout.split(frame.size()).as_ref() else {
        unreachable!()
    };

    // Render the filtered content
    {
        let block = Block::bordered();
        let para = Paragraph::new(app.filtered.as_str())
            .block(block);
        frame.render_widget(para, filtered_content);
    }

    // Render the jq error (if any)
    if let Some(err) = app.error.as_ref() {
        let border_style = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);

        let block = Block::bordered()
            .title("Error")
            .padding(Padding::horizontal(4))
            .border_style(border_style);

        let para = Paragraph::new(err.as_str())
            .block(block);
        frame.render_widget(para, errors);
    }
}
