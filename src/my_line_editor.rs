// allowing dead code in case the text_area crate is insufficient and we need to roll our own
#![allow(dead_code)]

#[derive(Debug)]
pub struct LineEditor {
    pub content: String,
    // The cursor position, 
    pub cursor_pos: usize,
}
impl LineEditor {
    pub fn from(content: String) -> Self {
        LineEditor {
            cursor_pos: content.len(),
            content,
        }
    }
    pub fn content(&self) -> &str {
        self.content.as_str()
    }
    pub fn insert_char(&mut self, ch: char) {
        self.content.insert(self.cursor_pos, ch);
        self.move_cursor_right();
    }
    pub fn cursor_byte_index(&mut self) -> usize {
        self.content
            .char_indices()
            .map(|(start, _ch)| start)
            .nth(self.cursor_pos)
            .unwrap_or(self.content.len())
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos == 0 {
            // nothing to do
            return;
        }
        // Method "remove" is not used on the saved text for deleting the selected char.
        // Reason: Using remove on String works on bytes instead of the chars.
        // Using remove would require special care because of char boundaries.

        let current_index = self.cursor_pos;
        let from_left_to_current_index = current_index - 1;

        // Getting all characters before the selected character.
        let before_char_to_delete = self.content.chars().take(from_left_to_current_index);
        // Getting all characters after selected character.
        let after_char_to_delete = self.content.chars().skip(current_index);

        // Put all characters together except the selected one.
        // By leaving the selected one out, it is forgotten and therefore deleted.
        self.content = before_char_to_delete.chain(after_char_to_delete).collect();
        self.move_cursor_left();
    }
    pub fn move_cursor_left(&mut self) {
        // move to the left, but don't go under 0
        let new_pos = self.cursor_pos.saturating_sub(1);
        let new_pos = new_pos.clamp(0, self.content.chars().count());
        self.cursor_pos = new_pos;
    }
    pub fn move_cursor_right(&mut self) {
        // move to the right, but don't go over the length
        let new_pos = self.cursor_pos.saturating_add(1);
        let new_pos = new_pos.clamp(0, self.content.chars().count());
        self.cursor_pos = new_pos;
    }
}
