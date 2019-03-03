use crate::{coord::*, idx::*};
use ropey::Rope;

/// Selection with `Coord`
///
/// An ordererd pair of indices in the buffer
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionUnaligned {
    pub anchor: Coord,
    pub cursor: Coord,
    pub was_forward: bool,
}

impl SelectionUnaligned {
    pub fn update_last_direction(mut self) -> Self {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            self.was_forward = true;
        } else if cursor < anchor {
            self.was_forward = false;
        }

        self
    }

    pub fn sorted(self) -> (Coord, Coord) {
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
            was_forward: self.was_forward,
        }
    }

    pub fn line_trimed(self, text: &Rope) -> Self {
        Self {
            anchor: self.anchor.trim_line_to_buf(text),
            cursor: self.cursor.trim_line_to_buf(text),
            was_forward: self.was_forward,
        }
    }

    /// Colapse anchor to the cursor
    pub fn collapsed(self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.cursor,
            was_forward: self.was_forward,
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
            was_forward: !self.was_forward,
        }
    }

    pub fn from_selection(sel: Selection, text: &Rope) -> Self {
        Self {
            cursor: sel.cursor.to_coord(text),
            anchor: sel.anchor.to_coord(text),
            was_forward: sel.was_forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Selection with coordinates aligned
///
/// As coordinates are aligned, it's OK to keep
/// just the index in the text.
pub struct Selection {
    pub anchor: Idx,
    pub cursor: Idx,
    pub was_forward: bool,
}

impl Selection {
    pub fn update_last_direction(mut self) -> Self {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            self.was_forward = true;
        } else if cursor < anchor {
            self.was_forward = false;
        }

        self
    }
    pub fn aligned(mut self, text: &Rope) -> Self {
        self.anchor = self.anchor.trim_to_text(text);
        self.cursor = self.cursor.trim_to_text(text);
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
            if self.was_forward {
                self.cursor = self.cursor.backward(text);
            } else {
                self.cursor = self.cursor.forward(text);
            }
            self.aligned(text)
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
            self.was_forward
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
            was_forward: self.was_forward,
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
            was_forward: !self.was_forward,
        }
    }

    pub fn is_empty(self) -> bool {
        self.cursor == self.anchor
    }
}
