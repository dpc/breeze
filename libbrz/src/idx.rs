use ropey::Rope;

use crate::position::*;
use crate::range::Range;
use crate::util::char;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum CharCategory {
    Alphanumeric,
    Whitespace,
    Punctuation,
}

fn char_category(ch: char) -> CharCategory {
    use self::CharCategory::*;
    if char::is_word_forming(ch) {
        Alphanumeric
    } else if ch.is_whitespace() {
        Whitespace
    } else {
        Punctuation
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Idx(pub usize);

impl Idx {
    pub fn begining(_: &Rope) -> Idx {
        Idx(0)
    }
    pub fn end(text: &Rope) -> Idx {
        Idx(text.len_chars())
    }

    pub fn backward(self, text: &Rope) -> Self {
        self.backward_n(1, text)
    }

    pub fn backward_n(self, n: usize, _text: &Rope) -> Self {
        Idx(self.0.saturating_sub(n))
    }

    pub fn forward_n(self, n: usize, text: &Rope) -> Self {
        Idx(std::cmp::min(self.0.saturating_add(n), text.len_chars()))
    }

    pub fn forward(self, text: &Rope) -> Self {
        self.forward_n(1, text)
    }
    pub fn to_position(self, text: &Rope) -> Position {
        Position::from_idx(self, text)
    }

    pub fn prev_char(self, text: &Rope) -> Option<char> {
        if self.0 == 0 {
            None
        } else {
            Some(text.char(self.0 - 1))
        }
    }

    pub fn next_char(self, text: &Rope) -> Option<char> {
        if self.0 >= text.len_chars() {
            None
        } else {
            Some(text.char(self.0))
        }
    }

    pub fn backward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = self;

        cur = cur.backward_while(char::is_newline, text);
        cur = cur.backward_while(char::is_non_newline_whitespace, text);

        let start = cur;
        if let Some(start_ch_category) = start.prev_char(text).map(char_category) {
            cur = cur.backward_while(|ch| char_category(ch) == start_ch_category, text);
        }

        (start, cur)
    }

    pub fn forward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = self;

        cur = cur.forward_while(char::is_newline, text);

        let start = cur;
        if let Some(start_ch_category) = start.next_char(text).map(char_category) {
            cur = cur.forward_while(|ch| char_category(ch) == start_ch_category, text);
        }

        cur = cur.forward_while(char::is_non_newline_whitespace, text);

        (start, cur)
    }

    pub fn backward_while(self, mut f: impl FnMut(char) -> bool, text: &Rope) -> Self {
        let mut cur = self;
        while cur.prev_char(text).map(&mut f).unwrap_or(false) {
            cur = cur.backward(text);
        }
        cur
    }

    pub fn forward_while(self, mut f: impl FnMut(char) -> bool, text: &Rope) -> Self {
        let mut cur = self;
        while cur.next_char(text).map(&mut f).unwrap_or(false) {
            cur = cur.forward(text);
        }
        cur
    }

    pub fn forward_to_line_end(self, text: &Rope) -> Idx {
        self.forward_while(char::is_not_newline, text)
    }

    pub fn forward_past_line_end(self, text: &Rope) -> Idx {
        self.forward_to_line_end(text).forward(text)
    }

    pub fn backward_to_line_start(self, text: &Rope) -> Idx {
        self.backward_while(|ch| ch != '\n', text)
    }

    pub fn down_unaligned(self, n: usize, column: Option<usize>, text: &Rope) -> Self {
        let mut coord = self.to_position(text);
        coord.line = coord.line.saturating_add(n);
        if let Some(column) = column {
            coord.column = column;
        }
        coord.trim_line_to_buf(text).to_idx(text)
    }

    pub fn up_unaligned(self, n: usize, column: Option<usize>, text: &Rope) -> Self {
        let mut coord = self.to_position(text);
        coord.line = coord.line.saturating_sub(n);
        if let Some(column) = column {
            coord.column = column;
        }
        coord.trim_line_to_buf(text).to_idx(text)
    }

    pub fn before_first_non_whitespace(self, text: &Rope) -> Self {
        self.backward_to_line_start(text)
            .forward_while(char::is_non_newline_whitespace, text)
    }

    pub fn trim_to_text(mut self, text: &Rope) -> Self {
        if self.0 > text.len_chars() {
            self.0 = text.len_chars();
        }
        self
    }

    pub fn range_to(self, other: Idx) -> Range {
        Range {
            from: self,
            to: other,
        }
    }
}

// Note: does not check bounds
impl std::ops::AddAssign<usize> for Idx {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

// Note: does not check bounds
impl std::ops::SubAssign<usize> for Idx {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

// Note: does not check bounds
impl From<usize> for Idx {
    fn from(u: usize) -> Self {
        Idx(u)
    }
}

// Note: does not check bounds
impl From<Idx> for usize {
    fn from(idx: Idx) -> Self {
        idx.0
    }
}
