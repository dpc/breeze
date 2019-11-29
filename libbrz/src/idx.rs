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

fn is_indent_opening_char(ch: char) -> bool {
    match ch {
        '{' | '(' | '[' | '<' | '"' => true,
        _ => false,
    }
}

fn is_indent_closing_char(ch: char) -> bool {
    match ch {
        '}' | ')' | ']' | '>' | '"' => true,
        _ => false,
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
        if self == Self::begining(text) {
            None
        } else {
            Some(text.char(self.0 - 1))
        }
    }

    pub fn next_char(self, text: &Rope) -> Option<char> {
        if self >= Self::end(text) {
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

    pub fn to_after_indent_opening_char(mut self, text: &Rope) -> Option<Idx> {
        let mut nesting = 0;
        loop {
            if self == Self::begining(text) {
                return None;
            }

            let char_before = self.prev_char(text).unwrap();

            if is_indent_opening_char(char_before) {
                if nesting == 0 {
                    return Some(self);
                } else {
                    nesting -= 1;
                }
            } else if is_indent_closing_char(char_before) {
                nesting += 1;
            }

            self = self.backward(text);
        }
    }

    pub fn to_before_indent_closing_char(mut self, text: &Rope) -> Option<Idx> {
        let mut nesting = 0;
        loop {
            if self == Self::end(text) {
                return None;
            }

            let char_before = self.next_char(text).unwrap();

            if is_indent_closing_char(char_before) {
                if nesting == 0 {
                    return Some(self);
                } else {
                    nesting -= 1;
                }
            } else if is_indent_opening_char(char_before) {
                nesting += 1;
            }

            self = self.forward(text);
        }
    }
    /// Desired indent when opening a line when on position `self`
    ///
    /// `bool` - increase indent
    pub fn desired_indent_when_opening_line(&self, text: &Rope) -> (Rope, bool) {
        // TODO: this could finish faster then go to the begining of the buffer (potentially)
        if let Some(indent_opening) = self.to_after_indent_opening_char(text) {
            let increase_indent =
                indent_opening.to_position(text).line == self.to_position(text).line;

            let line_begining = self.backward_to_line_start(text);
            let indent_end = self.before_first_non_whitespace(text);
            (
                line_begining.range_to(indent_end).slice(text).into(),
                increase_indent,
            )
        } else {
            (Rope::new(), false)
        }
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
