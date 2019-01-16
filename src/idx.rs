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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Idx(pub usize);

impl Idx {
    pub fn backward(self, _text: &Rope) -> Self {
        Idx(self.0.saturating_sub(1))
    }

    pub fn backward_n(self, n: usize, _text: &Rope) -> Self {
        Idx(self.0.saturating_sub(n))
    }
    pub fn forward(self, text: &Rope) -> Self {
        self.forward_n(1, text)
    }

    pub fn forward_n(self, n: usize, text: &Rope) -> Self {
        Idx(std::cmp::min(self.0.saturating_add(n), text.len_chars()))
    }

    pub fn to_coord(self, text: &Rope) -> Coord {
        Coord::from_idx(self, text)
    }

    pub fn backward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = std::cmp::min(self.0, text.len_chars().saturating_sub(1));

        loop {
            if cur == 0 {
                break;
            }
            let prev = cur.saturating_sub(1);
            let ch = text.char(prev);
            if ch.is_whitespace() {
                cur -= 1;
            } else {
                break;
            }
        }
        let start = cur;
        loop {
            if cur == 0 {
                break;
            }
            let prev = cur.saturating_sub(1);
            let ch = text.char(prev);
            if char_category(text.char(start.saturating_sub(1))) == char_category(ch) {
                cur -= 1;
            } else {
                break;
            }
        }
        (Idx(start), Idx(cur))
    }

    pub fn forward_word(self, text: &Rope) -> (Idx, Idx) {
        let mut cur = self.0;
        let text_len = text.len_chars();

        loop {
            if cur == text_len {
                break;
            }
            let ch = text.char(cur);
            if ch == '\n' {
                cur += 1;
            } else {
                break;
            }
        }
        let start = cur;

        loop {
            if cur == text_len {
                break;
            }
            let ch = text.char(cur);
            if char_category(text.char(start)) == char_category(ch) {
                cur += 1;
            } else {
                break;
            }
        }
        loop {
            if cur == text_len {
                break;
            }
            let ch = text.char(cur);
            if ch.is_whitespace() && ch != '\n' {
                cur += 1;
            } else {
                break;
            }
        }
        (Idx(start), Idx(cur))
    }

    pub fn forward_to_line_end(self, text: &Rope) -> Idx {
        let mut cur = self.0;
        let text_len = text.len_chars();
        if cur == text_len || text.char(cur) == '\n' {
            // nothing
        } else {
            cur += 1;
        }
        loop {
            if cur == text_len || text.char(cur) == '\n' {
                break;
            }
            cur += 1;
        }
        Idx(cur)
    }

    pub fn forward_past_line_end(self, text: &Rope) -> Idx {
        self.forward_to_line_end(text).forward(text)
    }

    pub fn backward_to_line_start(self, text: &Rope) -> Idx {
        let mut cur = std::cmp::min(self.0, text.len_chars().saturating_sub(1));
        loop {
            if cur == 0 {
                break;
            }
            let prev = cur.saturating_sub(1);
            if text.char(prev) == '\n' {
                break;
            }
            cur -= 1;
        }
        Idx(cur)
    }
}

impl From<usize> for Idx {
    fn from(u: usize) -> Self {
        Idx(u)
    }
}

impl From<Idx> for usize {
    fn from(idx: Idx) -> Self {
        idx.0
    }
}
