use ropey::Rope;

use crate::position::*;
use crate::range::Range;
use crate::util::char;
use std::collections::VecDeque;

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
        '{' | '(' | '[' | '<' | '"' | '\'' => true,
        _ => false,
    }
}

fn is_indent_closing_char(ch: char) -> bool {
    match ch {
        '}' | ')' | ']' | '>' | '"' | '\'' => true,
        _ => false,
    }
}

fn matching_char(ch: char) -> char {
    match ch {
        '{' => '}',
        '}' => '{',
        '[' => ']',
        ']' => '[',
        '<' => '>',
        '>' => '<',
        '(' => ')',
        ')' => '(',
        other => other,
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
        let mut nesting = vec![];

        loop {
            if self == Self::begining(text) {
                return None;
            }

            let ch = self.prev_char(text).unwrap();

            if is_indent_opening_char(ch) {
                if nesting.is_empty() {
                    return Some(self);
                } else if ch == matching_char(nesting[nesting.len() - 1]) {
                    nesting.pop();
                    self = self.backward(text);
                    continue;
                }
            }

            if is_indent_closing_char(ch) {
                nesting.push(ch);
            }

            self = self.backward(text);
        }
    }

    pub fn find_surounding_area(self, text: &Rope) -> (Idx, Idx) {
        self.find_surounding_area_opt(text)
            .unwrap_or_else(|| (Self::begining(text), Self::end(text)))
    }

    pub fn find_surounding_area_opt(self, text: &Rope) -> Option<(Idx, Idx)> {
        let begining = Idx::begining(text);
        let end = Idx::end(text);
        // left_back=====left_fron   right_front=====right_back
        let mut left_q: VecDeque<(char, Option<Idx>)> = VecDeque::new();
        let mut right_q: VecDeque<(char, Option<Idx>)> = VecDeque::new();

        let mut left = self;
        let mut right = self;
        let mut pushed_something = false;

        loop {
            if pushed_something && left_q.is_empty() && right_q.is_empty() {
                return Some((left.forward(text), right.backward(text)));
            }

            if left == begining && right == end {
                return None;
            }
            if left != Idx::begining(text) && (left_q.len() <= right_q.len() || right == end) {
                let ch = left.prev_char(text).unwrap();
                left = left.backward(text);

                match (is_indent_opening_char(ch), is_indent_closing_char(ch)) {
                    (true, false) => {
                        // we close the match if there is any, skipping any
                        // unmatched chars
                        let matching = matching_char(ch);
                        if right_q.iter().any(|(ch, _)| *ch == matching) {
                            loop {
                                let (ch, idx) = right_q.pop_front().expect("not empty");
                                if matching == ch {
                                    if let Some(idx) = idx {
                                        return Some((left.forward(text), idx.backward(text)));
                                    } else {
                                        break;
                                    }
                                }
                            }
                        } else {
                            left_q.push_back((ch, Some(left)));
                            pushed_something = true;
                        }
                    }
                    (false, true) => {
                        right_q.push_front((ch, None));
                    }
                    (true, true) => {
                        // we close the match if there is any, skipping any
                        // unmatched chars
                        let matching = matching_char(ch);
                        if right_q.iter().any(|(ch, _)| *ch == matching) {
                            loop {
                                let (ch, idx) = right_q.pop_front().expect("not empty");
                                if matching == ch {
                                    if let Some(idx) = idx {
                                        return Some((left.forward(text), idx.backward(text)));
                                    } else {
                                        break;
                                    }
                                }
                            }
                        } else {
                            left_q.push_front((ch, Some(left)));
                            pushed_something = true;
                        }
                    }
                    (false, false) => {}
                }
            } else if right != end {
                let ch = right.next_char(text).unwrap();
                right = right.forward(text);

                match (is_indent_closing_char(ch), is_indent_opening_char(ch)) {
                    (true, false) => {
                        // we close the match if there is any, skipping any
                        // unmatched chars
                        let matching = matching_char(ch);
                        if left_q.iter().any(|(ch, _)| *ch == matching) {
                            loop {
                                let (ch, idx) = left_q.pop_front().expect("not empty");

                                if matching == ch {
                                    if let Some(idx) = idx {
                                        return Some((idx.forward(text), right.backward(text)));
                                    } else {
                                        break;
                                    }
                                }
                            }
                        } else {
                            right_q.push_back((ch, Some(right)));
                            pushed_something = true;
                        }
                    }
                    (false, true) => {
                        left_q.push_front((ch, None));
                    }
                    (true, true) => {
                        // we close the match if there is any, skipping any
                        // unmatched chars

                        let matching = matching_char(ch);
                        if left_q.iter().any(|(ch, _)| *ch == matching) {
                            loop {
                                let (ch, idx) = left_q.pop_front().expect("not empty");

                                if matching == ch {
                                    if let Some(idx) = idx {
                                        return Some((idx.forward(text), right.backward(text)));
                                    } else {
                                        break;
                                    }
                                }
                            }
                        } else {
                            right_q.push_front((ch, Some(right)));
                            pushed_something = true;
                        }
                    }
                    (false, false) => {}
                }
            }
        }
    }

    pub fn to_before_indent_closing_char(mut self, text: &Rope) -> Option<Idx> {
        let mut nesting = vec![];
        loop {
            if self == Self::end(text) {
                return None;
            }

            let ch = self.next_char(text).unwrap();

            if is_indent_closing_char(ch) {
                if nesting.is_empty() {
                    return Some(self);
                } else if ch == matching_char(nesting[nesting.len() - 1]) {
                    nesting.pop();
                    self = self.forward(text);
                    continue;
                }
            }

            if is_indent_opening_char(ch) {
                nesting.push(ch);
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
