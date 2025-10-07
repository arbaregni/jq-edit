
use std::borrow::Cow;

use anyhow::Result;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{
        block::BlockExt,
        Block,
        Widget
    }
};

use crate::{json::JsonPathElement, streaming::Streaming};
use crate::json::{
    JsonData,
    JsonPath,
    JsonFragment
};

pub fn render_to_lines<'a, S>(json_stream: &mut S, lines: &mut Vec<Line<'a>>, line_cap: usize) -> Result<()>
    where S: Streaming<Item = (JsonPath<'a>, JsonFragment<'a>)>
{
    log::info!("rendering to lines...");

    let mut line_renderer = LineRenderer::from(lines);

    let mut lines_rendered = 0;

    while let Some((next_path, json_fragment)) = json_stream.try_next()? {
        log::debug!("received json fragment {json_fragment:?}");

        use crate::json::JsonFragment::*;

        line_renderer.write_indent();

        if let Some(JsonPathElement::PropertyInObject { key }) = next_path.last() {
            line_renderer
                .write_styled(enquote::enquote('"', key.to_lexeme()), Style::default().fg(Color::Blue))
                .write_punctuation(": ");
        }

        match json_fragment {
            BeginDict => {
                line_renderer
                    .write_punctuation("{")
                    .increase_indent()
            }
            EndDict => {
                line_renderer
                    .decrease_indent()
                    .write_punctuation("}")
            }
            BeginArray => {
                line_renderer
                    .write_punctuation("[")
                    .increase_indent()
            }
            EndArray => {
                line_renderer
                    .decrease_indent()
                    .write_punctuation("]")
            },
            Atom(JsonData::Null) => {
                line_renderer.write_styled("null", Style::default().fg(Color::LightMagenta))
            }
            Atom(JsonData::Str { value }) => {
                line_renderer.write_styled(enquote::enquote('"', &value), Style::default().fg(Color::Green))
            }
            Atom(JsonData::Number { value }) => {
                line_renderer.write_styled(format!("{value}"), Style::default().fg(Color::LightBlue))
            }
            Atom(JsonData::Float { value }) => {
                line_renderer.write_styled(format!("{value}"), Style::default().fg(Color::LightBlue))
            }
            Atom(JsonData::Boolean { value }) => {
                line_renderer.write_styled(format!("{value}"), Style::default().fg(Color::Yellow))
            }
            Invalid(invalid) => {
                line_renderer.write_styled(invalid, Style::default().bg(Color::Red).fg(Color::White))
            }
            Atom(json) => todo!("handle formatting {json:?}")
        };
        line_renderer.finish_line();

        lines_rendered += 1;
        if lines_rendered >= line_cap {
            break;
        }
    }

    line_renderer.flush();

    Ok(())
}

use line_renderer::LineRenderer;

mod line_renderer {
    use ratatui::{style::Color, text::Span};

    use super::*;

    const TAB: &'static str = "    "; // four spaces

    pub struct LineRenderer<'borrow, 'data> {
        indent: usize,
        lines: &'borrow mut Vec<Line<'data>>,
        curr_line: Vec<Span<'data>>
    }
    impl <'borrow, 'data> LineRenderer<'borrow, 'data> {
        pub fn from(lines: &'borrow mut Vec<Line<'data>>) -> Self {
            Self {
                indent: 0,
                curr_line: vec![],
                lines
            }
        }

        // public mutators to write parts of objects

        // Helpers for writing
        pub fn write_indent(&mut self) -> &mut Self {
            for _ in 0..self.indent {
                self.write_default(TAB);
            }
            self
        }
        pub fn increase_indent(&mut self) -> &mut Self {
            self.indent = self.indent.saturating_add(1);
            self
        }
        pub fn decrease_indent(&mut self) -> &mut Self {
            self.indent = self.indent.saturating_sub(1);
            self
        }


        pub fn write_punctuation(&mut self, string: &'data str) -> &mut Self {
            self.write_styled(string, Style::default())
        }

       pub fn write_styled<S: Into<Cow<'data, str>>>(&mut self, string: S, style: Style) -> &mut Self {
            self.write_span(Span::styled(string, style))
        }

        pub fn write_default(&mut self, string: &'data str) -> &mut Self {
            self.write_span(Span::from(string))
        }

        pub fn write_span(&mut self, span: Span<'data>) -> &mut Self {
            self.curr_line.push(span);
            self
        }
        pub fn finish_line(&mut self) -> &mut Self {
            let line = Line::from(self.curr_line.clone());
            self.lines.push(line);
            self.curr_line.clear();
            self
        }
        pub fn flush(&mut self) -> &mut Self {
            if self.curr_line.len() == 0 {
                return self;
            }
            let mut curr_line = Vec::new();
            std::mem::swap(&mut curr_line, &mut self.curr_line); // this clears self.curr_line and
                                                                 // gives us ownership of the array
            let line = Line::from(curr_line);
            self.lines.push(line);
            self
        }

    }
}

