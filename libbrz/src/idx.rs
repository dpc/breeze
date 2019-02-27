use ropey::Rope;

use crate::coord::*;

fn char_is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum CharCategory {
    Alphanumeric,
    Whitespace,
    Punctuation,
}

fn char_category(ch: char) -> CharCategory {
    use self::CharCategory::*;
    if char_is_word(ch) {
        Alphanumeric
    } else if ch.is_whitespace() {
        Whitespace
    } else {
        Punctuation
    }
}

fn char_is_newline(ch: char) -> bool {
    ch == '\n'
}

pub fn char_is_not_newline(ch: char) -> bool {
    ch != '\n'
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Idx(pub usize);

impl Idx {
    pub fn backward(self, n: usize) -> Self {
        Idx(self.0.saturating_sub(n))
    }
    pub fn forward(self, n: usize, text: &Rope) -> Self {
        Idx(std::cmp::min(self.0.saturating_add(n), text.len_chars()))
    }

    pub fn to_coord(self, text: &Rope) -> Coord {
        Coord::from_idx(self, text)
    }

    pub fn backward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = self;

        while cur
            .leftside_char(text)
            .map(char::is_whitespace)
            .unwrap_or(false)
        {
            cur -= 1;
        }

        let start = cur;
        let start_ch_category = start.leftside_char(text).map(char_category);

        while cur.leftside_char(text).map(char_category) == start_ch_category
            && start_ch_category.is_some()
        {
            cur -= 1;
        }

        (start, cur)
    }

    pub fn leftside_char(self, text: &Rope) -> Option<char> {
        if self.0 == 0 {
            None
        } else {
            Some(text.char(self.0 - 1))
        }
    }

    pub fn rightside_char(self, text: &Rope) -> Option<char> {
        if self.0 >= text.len_chars() {
            None
        } else {
            Some(text.char(self.0))
        }
    }

    pub fn forward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = self;

        while cur.rightside_char(text) == Some('\n') {
            cur += 1;
        }

        let start = cur;
        let start_ch_category = start.rightside_char(text).map(char_category);

        while cur.rightside_char(text).map(char_category) == start_ch_category
            && start_ch_category.is_some()
        {
            cur += 1;
        }

        while cur
            .rightside_char(text)
            .map(|ch| ch.is_whitespace() && ch != '\n')
            .unwrap_or(false)
        {
            cur += 1;
        }
        (start, cur)
    }

    pub fn forward_to_line_end(self, text: &Rope) -> Idx {
        let mut cur = self;

        while cur
            .rightside_char(text)
            .map(char_is_not_newline)
            .unwrap_or(false)
        {
            cur += 1;
        }

        cur
    }

    pub fn forward_past_line_end(self, text: &Rope) -> Idx {
        self.forward_to_line_end(text).forward(1, text)
    }

    pub fn backward_to_line_start(self, text: &Rope) -> Idx {
        let mut cur = self;

        while cur
            .leftside_char(text)
            .map(|ch| ch != '\n')
            .unwrap_or(false)
        {
            cur -= 1;
        }
        cur
    }

    pub fn down_unaligned(self, n: usize, column: Option<usize>, text: &Rope) -> Self {
        let mut coord = self.to_coord(text);
        coord.line = coord.line.saturating_add(n);
        if let Some(column) = column {
            coord.column = column;
        }
        coord.trim_line_to_buf(text).to_idx(text)
    }

    pub fn up_unaligned(self, n: usize, column: Option<usize>, text: &Rope) -> Self {
        let mut coord = self.to_coord(text);
        coord.line = coord.line.saturating_sub(n);
        if let Some(column) = column {
            coord.column = column;
        }
        coord.trim_line_to_buf(text).to_idx(text)
    }

    pub fn before_first_non_whitespace(self, text: &Rope) -> Self {
        self.to_coord(text)
            .after_leading_whitespace(text)
            .to_idx(text)
    }

    pub fn saturating_add(self, n: usize) -> Self {
        Idx(self.0.saturating_add(n))
    }

    pub fn saturating_sub(self, n: usize) -> Self {
        Idx(self.0.saturating_sub(n))
    }

    pub fn aligned(mut self, text: &Rope) -> Self {
        if self.0 > text.len_chars() {
            self.0 = text.len_chars();
        }
        self
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
