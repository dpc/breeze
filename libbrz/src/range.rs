use crate::idx::Idx;

use ropey::{Rope, RopeSlice};
pub struct Range {
    pub from: Idx,
    pub to: Idx,
}

impl Range {
    pub fn sorted(self) -> Self {
        if self.from > self.to {
            self.reversed()
        } else {
            self
        }
    }

    pub fn reversed(self) -> Self {
        Self {
            to: self.from,
            from: self.to,
        }
    }

    pub fn slice(self, text: &Rope) -> RopeSlice {
        let sorted = self.sorted();
        text.slice(sorted.from.0..sorted.to.0)
    }
}
