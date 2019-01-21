use ropey::Rope;

use crate::idx::*;
use std::cmp::min;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Coordinate where the column can exceed the line length
pub struct CoordUnaligned {
    pub line: usize,
    pub column: usize,
}

impl From<Coord> for CoordUnaligned {
    fn from(coord: Coord) -> Self {
        Self {
            line: coord.line,
            column: coord.column,
        }
    }
}

impl CoordUnaligned {
    fn map_as_coord<F>(self, text: &Rope, f: F) -> Self
    where
        F: FnOnce(Coord, &Rope) -> Coord,
    {
        f(self.trim_column_to_buf(text), text).into()
    }

    fn map_as_coord_2<F>(self, text: &Rope, f: F) -> (Self, Self)
    where
        F: FnOnce(Coord, &Rope) -> (Coord, Coord),
    {
        let (a, b) = f(self.trim_column_to_buf(text), text);
        (a.into(), b.into())
    }

    pub fn set_line(mut self, line: usize, text: &Rope) -> Self {
        self.line = line;
        self.trim_line_to_buf(text)
    }

    pub fn set_column(mut self, column: usize, text: &Rope) -> Self {
        self.column = column;
        self.trim_column_to_buf(text).into()
    }

    pub fn to_idx(self, text: &Rope) -> Idx {
        self.trim_column_to_buf(text).to_idx(text)
    }

    pub fn forward(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::forward)
    }

    pub fn forward_n(self, n: usize, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_n(n, text))
    }

    pub fn forward_to_line_end(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_to_line_end(text))
    }

    pub fn forward_past_line_end(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_past_line_end(text))
    }

    pub fn backward(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::backward)
    }

    pub fn backward_n(self, n: usize, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.backward_n(n, text))
    }

    pub fn backward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_coord_2(text, Coord::backward_word)
    }

    pub fn backward_to_line_start(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.backward_to_line_start(text))
    }

    pub fn forward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_coord_2(text, Coord::forward_word)
    }

    pub fn up_unaligned(self, _text: &Rope) -> Self {
        Self {
            line: self.line.saturating_sub(1),
            column: self.column,
        }
    }

    pub fn down_unaligned(self, text: &Rope) -> Self {
        let lines = text.len_lines();
        Self {
            line: if self.line == lines || self.line + 1 == lines {
                self.line
            } else {
                self.line + 1
            },
            column: self.column,
        }
    }

    /// Align to buffer
    ///
    /// Column in the `Coord` can actually exeed the actual column,
    /// which is useful eg. for consecutive up and down movements
    pub fn trim_column_to_buf(self, text: &Rope) -> Coord {
        let line = text.line(self.line);
        let line_len = line.len_chars();
        let trimed_column = if line_len == 0 {
            0
        } else if self.line + 1 == text.len_lines() {
            std::cmp::min(self.column, line_len)
        } else {
            std::cmp::min(self.column, line_len - 1)
        };

        Coord {
            line: self.line,
            column: trimed_column,
        }
    }

    pub fn trim_line_to_buf(self, text: &Rope) -> Self {
        Self {
            column: self.column,
            line: min(self.line, text.len_lines().saturating_sub(1)),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
/// Coordinate where the row is known to be within the line length
///
/// Note: This is within the buffer this `Coord` was created to work
/// in.
pub struct Coord {
    pub line: usize,
    pub column: usize,
}

impl Coord {
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

    pub fn to_idx(self, text: &Rope) -> Idx {
        (text.line_to_char(self.line) + self.column).into()
    }

    pub fn from_idx(idx: Idx, text: &Rope) -> Self {
        let line = text.char_to_line(idx.0);
        let line_start_pos = text.line_to_char(line);
        let column = idx.0 - line_start_pos;

        Self { line, column }
    }

    pub fn forward(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward(text), text)
    }

    pub fn forward_n(self, n: usize, text: &Rope) -> Self {
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

    pub fn backward(self, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.backward(text))
    }

    pub fn backward_n(self, n: usize, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.backward_n(n, text))
    }

    pub fn forward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_idx_2(text, |idx| idx.forward_word(text))
    }

    pub fn backward_word(self, text: &Rope) -> (Self, Self) {
        self.map_as_idx_2(text, |idx| idx.backward_word(text))
    }
}
