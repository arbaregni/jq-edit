use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, style::{
        Color, Modifier, Style
    }, terminal::Frame, text::{Line, Span, Text}, widgets::{Block, Borders, Padding, Paragraph}
};

use crate::{
    app::{App, ErrorPanel},
    parse::{Token, TokenType}, scroll_text::ScrollText,
};


pub fn render_app(app: &App, frame: &mut Frame) {
    // the number of lines to spend on error message
    let error_len = match app.error.as_ref() {
        None => 0,
        Some(err) => err.failure.lines().count().clamp(4, 64) as u16
    };
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), Constraint::Length(error_len), Constraint::Length(5)]
    );
    let &[filtered_content, error_messages, query_edit] = layout.split(frame.size()).as_ref() else {
        panic!("wrong number of values to unpack during layout")
    };

    // Render the jq error (if any)
    if let Some(err) = app.error.as_ref() {
        render_error_panel(err, frame, error_messages);
    }

    // Render the filtered content
    {
        // TODO: not every frame please !
        // let tokens = parse::tokenize(&app.filtered);
        // let mut text = Text::default();
        // tokens_to_text(&tokens, &mut text);

        // let text = Paragraph::new(app.filtered.as_str())
        //    .block(block);

        let block = Block::bordered();

        let w = app.scroll_text.widget()
            .block(block);

        frame.render_widget(w, filtered_content);
    }

    // render the current query
    {
        let w = app.query_editor.widget();
        frame.render_widget(w, query_edit);
    }

}

pub fn set_query_editor_styles(app: &mut App) {
    let line_style = Style::default();
    app.query_editor.set_cursor_line_style(line_style);

    let block_style = match &app.error {
        Some(_) => Style::default().fg(Color::Red),
        None => Style::default(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("jq")
        .padding(Padding::vertical(1))
        .style(block_style);

    app.query_editor.set_block(block);
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

pub fn tokens_to_text<'a>(tokens: &[Token<'a>], text: &mut Text<'a>) {

    text.lines.clear();
    let mut curr_line = Vec::new();

    for tok in tokens {

        let span = token_to_span(tok);
        curr_line.push(span);

        if tok.tty == TokenType::Newline {
            let line = Line::from(curr_line.clone());
            text.lines.push(line);
            curr_line.clear();
        }
        
    }
}

fn token_to_span<'a>(tok: &Token<'a>) -> Span<'a> {
    let style = match tok.tty {
        TokenType::OpenBrace | TokenType::CloseBrace  | TokenType::OpenBracket 
            | TokenType::CloseBracket  | TokenType::Comma  | TokenType::Colon  
            | TokenType::Whitespace  | TokenType::Newline => Style::default(),
        TokenType::String => Style::default().fg(Color::Green),
        TokenType::Number => Style::default().fg(Color::Yellow),
        TokenType::InvalidChar => Style::default().fg(Color::White).bg(Color::Red),
    };
    Span::styled(tok.lex, style)
}
