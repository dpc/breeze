use crate::{coord::*, idx::*};
use ropey::Rope;

/// Selection with `Coord`
///
/// An ordererd pair of indices in the buffer
#[derive(Default, Debug, Clone, Copy)]
pub struct SelectionUnaligned {
    pub anchor: Coord,
    pub cursor: Coord,
}

impl SelectionUnaligned {
    pub fn align(self, text: &Rope) -> Selection {
        Selection {
            anchor: self.anchor.trim_column_to_buf(text).to_idx(text),
            cursor: self.cursor.trim_column_to_buf(text).to_idx(text),
        }
    }

    pub fn trim(self, text: &Rope) -> Self {
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
            cursor: sel.cursor.to_coord(text).into(),
            anchor: sel.anchor.to_coord(text).into(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
/// Selection with coordinates aligned
///
/// As coordinates are aligned, it's OK to keep
/// just the index in the text.
pub struct Selection {
    pub anchor: Idx,
    pub cursor: Idx,
}

impl Selection {
    pub fn is_idx_inside(self, idx: Idx) -> bool {
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

    pub fn is_forward(self) -> Option<bool> {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            Some(true)
        } else if cursor < anchor {
            Some(false)
        } else {
            None
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
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
        }
    }
}
