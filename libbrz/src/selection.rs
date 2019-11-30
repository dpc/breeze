use crate::{idx::*, position::*};
use ropey::Rope;

/// Selection with `Position`
///
/// An ordererd pair of indices in the buffer
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionUnaligned {
    pub anchor: Position,
    pub cursor: Position,
}

impl SelectionUnaligned {
    pub fn sorted(self) -> (Position, Position) {
        if self.anchor.line < self.cursor.line {
            (self.anchor, self.cursor)
        } else if self.cursor.line < self.anchor.line {
            (self.cursor, self.anchor)
        } else if self.cursor.column < self.anchor.column {
            (self.cursor, self.anchor)
        } else {
            (self.anchor, self.cursor)
        }
    }

    pub fn is_empty(self, text: &Rope) -> bool {
        let aligned = self.aligned(text);
        aligned.cursor == aligned.anchor
    }

    pub fn aligned(self, text: &Rope) -> Selection {
        Selection {
            anchor: self.anchor.trim_column_to_buf(text).to_idx(text),
            cursor: self.cursor.trim_column_to_buf(text).to_idx(text),
        }
    }

    pub fn line_trimed(self, text: &Rope) -> Self {
        Self {
            anchor: self.anchor.trim_line_to_buf(text),
            cursor: self.cursor.trim_line_to_buf(text),
        }
    }

    /// Colapse anchor to the cursor
    pub fn collapsed(self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.cursor,
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
        }
    }

    pub fn from_selection(sel: Selection, text: &Rope) -> Self {
        Self {
            cursor: sel.cursor.to_position(text),
            anchor: sel.anchor.to_position(text),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Selection with coordinates normalized
///
/// As coordinates are normalized, it's OK to keep
/// just the index in the text.
pub struct Selection {
    pub anchor: Idx,
    pub cursor: Idx,
}

impl Selection {
    pub fn new_from_normalized(anchor: Idx, cursor: Idx) -> Self {
        Self { anchor, cursor }
    }

    pub fn new(anchor: Idx, cursor: Idx, text: &Rope) -> Self {
        Self { anchor, cursor }.normalized(text)
    }

    pub fn unify_direction_of(self, other: Self) -> Self {
        if self.is_forward() ^ other.is_forward() {
            other.reversed()
        } else {
            other
        }
    }

    pub fn normalized(mut self, text: &Rope) -> Self {
        self.anchor = self.anchor.trim_to_text(text);
        self.cursor = self.cursor.trim_to_text(text);
        if self.anchor == self.cursor {
            self.anchor = self.cursor.backward(text);
        }
        if self.anchor == self.cursor {
            self.anchor = self.cursor.forward(text);
        }
        self
    }

    pub fn is_idx_strictly_inside(self, idx: Idx) -> bool {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            anchor <= idx && idx < cursor
        } else if cursor < anchor {
            cursor <= idx && idx < anchor
        } else {
            false
        }
    }

    pub fn is_idx_inside_direction_marker(self, idx: Idx, text: &Rope) -> bool {
        if self.is_forward() {
            self.cursor == idx.forward_n(1, text)
        } else {
            self.cursor == idx
        }
    }

    pub fn self_or_direction_marker(mut self, text: &Rope) -> Self {
        if self.anchor == self.cursor {
            self.cursor = self.cursor.backward(text);
            self.normalized(text)
        } else {
            self
        }
    }

    pub fn is_forward(self) -> bool {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            true
        } else if cursor < anchor {
            false
        } else {
            true
        }
    }

    pub fn sorted(self) -> Self {
        if self.is_forward() {
            self
        } else {
            self.reversed()
        }
    }

    pub fn sorted_pair(self) -> (Idx, Idx) {
        if self.is_forward() {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
    pub fn sorted_range(self) -> std::ops::Range<Idx> {
        let (a, b) = self.sorted_pair();
        a..b
    }

    pub fn sorted_range_usize(self) -> std::ops::Range<usize> {
        let (a, b) = self.sorted_pair();
        a.into()..b.into()
    }

    /// Colapse anchor to the cursor
    pub fn collapsed(self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.cursor,
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
        }
    }

    pub fn is_empty(self) -> bool {
        self.cursor == self.anchor
    }
}
