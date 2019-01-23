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

#[derive(Debug, Clone, Copy)]
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
    pub fn aligned(mut self, text: &Rope) -> Self {
        self.anchor = self.anchor.aligned(text);
        self.cursor = self.cursor.aligned(text);
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

    pub fn is_idx_inside_direction_marker(self, idx: Idx) -> bool {
        if self.is_forward() {
            self.cursor == idx
        } else {
            self.cursor == idx.saturating_add(1)
        }
    }

    pub fn self_or_direction_marker(mut self, text: &Rope) -> Self {
        if self.anchor == self.cursor {
            if self.was_forward {
                self.cursor = self.cursor.saturating_add(1);
            } else {
                self.cursor = self.cursor.saturating_sub(1);
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

    pub fn sorted(self) -> (Idx, Idx) {
        if self.anchor < self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    pub fn sorted_range(self) -> std::ops::Range<Idx> {
        let (a, b) = self.sorted();
        a..b
    }

    pub fn sorted_range_usize(self) -> std::ops::Range<usize> {
        let (a, b) = self.sorted();
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
}
