use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{
        Color, Modifier, Style
    },
    terminal::Frame,
    widgets::{Block, Padding, Paragraph}
};

use crate::app::{App, ErrorPanel};


pub fn render_app(app: &App, frame: &mut Frame) {
    let error_len = match app.error.as_ref() {
        None => 0,
        Some(err) => err.failure.lines().count().clamp(4, 64) as u16
    };
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Length(error_len), Constraint::Fill(1), Constraint::Length(1)]
    );
    let &[error_messages, filtered_content, query_edit] = layout.split(frame.size()).as_ref() else {
        panic!("wrong number of values to unpack during layout")
    };

    // Render the jq error (if any)
    if let Some(err) = app.error.as_ref() {
        render_error_panel(err, frame, error_messages);
    }

    // Render the filtered content
    {
        let block = Block::bordered();
        let para = Paragraph::new(app.filtered.as_str())
            .block(block);
        frame.render_widget(para, filtered_content);
    }

}

fn render_error_panel(err: &ErrorPanel, frame: &mut Frame, size: Rect) {
    let border_style = Style::default()
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD);

    let block = Block::bordered()
        .title(err.title.as_str())
        .padding(Padding::horizontal(4))
        .border_style(border_style);

    let para = Paragraph::new(err.failure.as_str())
        .block(block);

    frame.render_widget(para, size);
}
