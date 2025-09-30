
use anyhow::Result;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{
        block::BlockExt,
        Block,
        Widget
    }
};

use crate::streaming::Streaming;
use crate::json::{
    JsonData,
    JsonPath
};

pub fn render_to_lines<'a, S>(json_stream: &mut S, lines: &mut Vec<Line<'a>>) -> Result<()>
    where S: Streaming<Item = (JsonPath<'a>, JsonData<'a>)>
{
    let mut idx = 0;
    let mut curr_path = JsonPath::empty();

    let mut line_renderer = LineRenderer::from(lines);

    while let Some((next_path, json_data)) = json_stream.try_next()? {

        use crate::json::JsonFragment::*;

       /* for token in curr_path.tokens_to(&next_path) {

            match token {
                BeginDict => {
                    line_renderer
                        .write_indent()
                        .write_punctuation("{")
                        .finish_line()
                        .increase_indent();
                }
                EndDict => {
                    line_renderer
                        .decrease_indent()
                        .write_indent()
                        .write_punctuation("}")
                        .finish_line();
                }
                BeginArray => {
                    line_renderer
                        .write_indent()
                        .write_punctuation("[")
                        .finish_line()
                        .increase_indent();
                }
                EndArray => {
                    line_renderer
                        .decrease_indent()
                        .write_indent()
                        .write_punctuation("]")
                        .finish_line();
                }
                NextItem => {
                    line_renderer
                        .write_punctuation(",")
                        .finish_line();
                },
                NextObject => {
                    /* nothing to do */
                }
            }


        }
    */



    }

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
        pub fn write_styled(&mut self, string: &'data str, style: Style) -> &mut Self {
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

    }
}

