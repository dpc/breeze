use ropey::Rope;

use crate::idx::*;
use std::cmp::min;

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Eq, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn map_as_idx<F>(self, text: &Rope, f: F) -> Self
    where
        F: FnOnce(Idx) -> Idx,
    {
        Self::from_idx(f(self.to_idx(text)), text)
    }

    pub fn map_as_idx_2<F>(self, text: &Rope, f: F) -> (Self, Self)
    where
        F: FnOnce(Idx) -> (Idx, Idx),
    {
        let (a, b) = f(self.to_idx(text));
        (Self::from_idx(a, text), Self::from_idx(b, text))
    }

    pub fn set_line(mut self, line: usize, text: &Rope) -> Self {
        self.line = line;
        self.trim_line_to_buf(text)
    }

    pub fn set_column(mut self, column: usize, text: &Rope) -> Self {
        self.column = column;
        self.trim_column_to_buf(text)
    }

    pub fn to_idx(self, text: &Rope) -> Idx {
        let trimed = self.trim_column_to_buf(text);
        (text.line_to_char(trimed.line) + trimed.column).into()
    }

    pub fn up_unaligned(self, n: usize, _text: &Rope) -> Self {
        Position {
            line: self.line.saturating_sub(n),
            column: self.column,
        }
    }

    pub fn down_unaligned(mut self, n: usize, text: &Rope) -> Self {
        self.line = self.line.saturating_add(n);
        self.trim_line_to_buf(text)
    }

    /// Align to buffer
    ///
    /// Column in the `Coord` can actually exeed the actual column,
    /// which is useful eg. for consecutive up and down movements
    pub fn trim_column_to_buf(self, text: &Rope) -> Position {
        let line = text.line(self.line);
        let line_len = line.len_chars();
        let trimed_column = if line_len == 0 {
            0
        } else if self.line + 1 == text.len_lines() {
            std::cmp::min(self.column, line_len)
        } else {
            std::cmp::min(self.column, line_len - 1)
        };

        Position {
            line: self.line,
            column: trimed_column,
        }
    }

    pub fn trim_line_to_buf(self, text: &Rope) -> Self {
        Position {
            column: self.column,
            line: min(self.line, text.len_lines().saturating_sub(1)),
        }
    }

    pub fn from_idx(idx: Idx, text: &Rope) -> Self {
        let line = text.char_to_line(idx.0);
        let line_start_pos = text.line_to_char(line);
        let column = idx.0 - line_start_pos;

        Position { line, column }
    }

    pub fn forward(self, n: usize, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_n(n, text), text)
    }

    pub fn forward_to_line_end(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_to_line_end(text), text)
    }

    pub fn forward_past_line_end(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_past_line_end(text), text)
    }

    pub fn backward_to_line_start(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).backward_to_line_start(text), text)
    }

    pub fn backward(self, n: usize, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.backward_n(n, text))
    }

    pub fn forward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_idx_2(text, |idx| idx.forward_word(text))
    }

    pub fn backward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_idx_2(text, |idx| idx.backward_word(text))
    }
}
