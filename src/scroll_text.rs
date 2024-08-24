use ratatui::{
    buffer::Buffer, layout::Rect, style::Style, text::Line, widgets::{block::BlockExt, Block, Widget}
};

use crate::{tokens::{self, Token, TokenType}, ui};

#[derive(Debug)]
pub struct ScrollText<'a> {
    // the lines to render
    lines: Vec<Line<'a>>,
    // the first line that it will actually render
    line_offset: usize,
    // the styles to use
}

impl <'a> ScrollText<'a> {
    pub fn from(content: String) -> ScrollText<'a> {
        // TODO: how to avoid all the copying here??
        ScrollText::from_content(content)
    }

    pub fn from_content(content: String) -> ScrollText<'a> {
        let lines = content.lines()
            .map(|l| Line::from(l.to_string()))
            .collect();

        Self {
            line_offset: 0,
            lines,
        } 
    }
    pub fn from_tokens<'b>(tokens: &[Token<'b>]) -> ScrollText<'a> {
        let mut lines = Vec::new();
        let mut curr_line = Vec::new();
        for tok in tokens {

            let span = ui::token_to_span(tok);

            curr_line.push(span);

            if tok.tty == TokenType::Newline {
                let line = Line::from(curr_line.clone());
                lines.push(line);
                curr_line.clear();
            }
        }

        Self {
            line_offset: 0,
            lines
        }
    }

    pub fn scroll_up(&mut self) {
        self.line_offset = self.line_offset.saturating_sub(1);
        log::info!("scrolled up, line_offset = {}", self.line_offset);
    }
    pub fn scroll_down(&mut self) {
        self.line_offset = self.line_offset.saturating_add(1);
        log::info!("scrolled down, line_offset = {}", self.line_offset);
    }

    pub fn widget<'b>(&'b self) -> ScrollTextRef<'a, 'b> {
        ScrollTextRef {
            scroll_text: self,
            style: Style::default(),
            block: None,
        }
    }

    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        
        // TODO: efficiency
        for (row_idx, row) in area.rows().enumerate() {
            let idx = row_idx + self.line_offset;
            let Some(line) = self.lines.get(idx) else { continue; };

            let x_offset = 0;

            let line_area = Rect {
                x: area.x + x_offset,
                y: row.y,
                width: area.width - x_offset,
                height: 1
            };
            line.render(line_area, buf);

        }
    }
}

pub struct ScrollTextRef<'a, 'b> {
    scroll_text: &'b ScrollText<'a>,
    block: Option<Block<'a>>,
    style: Style,
    
}
impl <'a, 'b> Widget for ScrollTextRef<'a, 'b> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut area = area.intersection(buf.area);
        buf.set_style(area, self.style);

        if let Some(block) = &self.block {
            block.render(area, buf);
            area = self.block.inner_if_some(area);
        }

        self.scroll_text.render_ref(area, buf);
    }
}
impl <'a, 'b> ScrollTextRef<'a, 'b> {
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}
