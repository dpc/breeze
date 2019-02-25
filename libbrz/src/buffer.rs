#![allow(dead_code)]
use crate::{coord::*, idx::*, prelude::*, selection::*};
use ropey::Rope;
use std::cmp::min;
use std::collections::BTreeSet;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VisualSelection {
    DirectionMarker,
    Selection,
    None,
}

pub fn distance_to_next_tabstop(visual_column: usize, tabstop: usize) -> usize {
    let next_tabstop = (visual_column + tabstop) / tabstop * tabstop;
    next_tabstop - visual_column
}

#[test]
fn distance_to_next_tabstop_test() {
    for (v_col, expected) in &[(0, 4), (1, 3), (2, 2), (3, 1), (4, 4)] {
        assert_eq!(distance_to_next_tabstop(*v_col, 4), *expected);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    pub primary: usize,
    pub selections: Vec<SelectionUnaligned>,
}

impl Default for SelectionSet {
    fn default() -> Self {
        let sel = SelectionUnaligned::default();
        Self {
            selections: vec![sel],
            primary: 0,
        }
    }
}

impl SelectionSet {
    pub fn to_lines(&self) -> BTreeSet<usize> {
        let mut lines = BTreeSet::new();
        for s in &self.selections {
            let (from, to) = s.sorted();
            for line in from.line..=to.line {
                lines.insert(line);
            }
        }
        lines
    }
}

/// Buffer
///
/// A file opened for edition + some state around
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub text: ropey::Rope,
    pub selection: SelectionSet,
    pub tabstop: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            text: Rope::default(),
            tabstop: 4,
            selection: default(),
        }
    }
}

impl Buffer {
    pub fn from_text(text: Rope) -> Self {
        Self { text, ..default() }
    }

    fn for_each_selection<F, R>(&self, mut f: F) -> Vec<R>
    where
        F: FnMut(&SelectionUnaligned, &Rope) -> R,
    {
        let Self {
            ref selection,
            ref text,
            ..
        } = *self;

        selection
            .selections
            .iter()
            .map(|sel| f(sel, text))
            .collect()
    }

    fn map_each_selection_mut<F, R>(&mut self, mut f: F) -> Vec<R>
    where
        F: FnMut(&mut SelectionUnaligned, &mut Rope) -> R,
    {
        let Self {
            ref mut selection,
            ref mut text,
            ..
        } = *self;

        selection
            .selections
            .iter_mut()
            .map(|sel| {
                let res = f(sel, text);
                *sel = sel.update_last_direction();
                res
            })
            .collect()
    }

    fn map_each_enumerated_selection<F, R>(&self, mut f: F) -> Vec<R>
    where
        F: FnMut(usize, &SelectionUnaligned, &Rope) -> R,
    {
        let Self {
            ref selection,
            ref text,
            ..
        } = *self;

        selection
            .selections
            .iter()
            .enumerate()
            .map(|(i, sel)| f(i, sel, text))
            .collect()
    }

    fn map_each_enumerated_selection_mut<F, R>(&mut self, mut f: F) -> Vec<R>
    where
        F: FnMut(usize, &mut SelectionUnaligned, &mut Rope) -> R,
    {
        let Self {
            ref mut selection,
            ref mut text,
            ..
        } = *self;

        selection
            .selections
            .iter_mut()
            .enumerate()
            .map(|(i, sel)| f(i, sel, text))
            .collect()
    }

    pub fn idx_selection_type(&self, idx: Idx) -> VisualSelection {
        if self
            .selection
            .selections
            .iter()
            .any(|sel| sel.aligned(&self.text).is_idx_strictly_inside(idx))
        {
            VisualSelection::Selection
        } else if self.selection.selections.iter().any(|sel| {
            sel.is_empty(&self.text) && sel.aligned(&self.text).is_idx_inside_direction_marker(idx)
        }) {
            VisualSelection::DirectionMarker
        } else {
            VisualSelection::None
        }
    }

    pub fn reverse_selections(&mut self) {
        self.map_each_selection_mut(|sel, _text| *sel = sel.reversed());
    }

    pub fn insert_char(&mut self, ch: char) {
        self.insert(&(ch.to_string()));
    }

    pub fn insert(&mut self, s: &str) {
        let mut insertion_points = self.map_each_enumerated_selection(|i, sel, text| {
            (i, sel.cursor.trim_column_to_buf(text).to_idx(text))
        });
        insertion_points.sort_by_key(|&(_, idx)| idx);
        insertion_points.reverse();

        // we insert from the back, fixing idx past the insertion every time
        // this is O(n^2) while it could be O(n)
        for (i, (_, idx)) in insertion_points.iter().enumerate() {
            self.text.insert(idx.0, s);
            for fixing_i in 0..=i {
                let fixing_sel = &mut self.selection.selections[insertion_points[fixing_i].0];
                fixing_sel.cursor = fixing_sel.cursor.forward(s.len(), &self.text);
                *fixing_sel = fixing_sel.collapsed();
            }
        }
    }
    pub fn open(&mut self) {
        let mut indents = self.map_each_enumerated_selection(|i, sel, text| {
            let line_begining = sel.cursor.backward_to_line_start(text).to_idx(text).0;
            let indent_end = sel.cursor.before_first_non_whitespace(text).to_idx(text).0;
            let indent: Rope = text.slice(line_begining..indent_end).into();
            let line_end = sel.cursor.forward_to_line_end(text);
            (i, indent, line_end)
        });
        indents.sort_by_key(|&(_, _, line_end)| line_end);
        indents.reverse();

        // we insert from the back, fixing idx past the insertion every time
        // this is O(n^2) while it could be O(n)
        for (i, (_, indent, line_end)) in indents.iter().enumerate() {
            self.text
                .insert(line_end.to_idx(&self.text).0, &indent.to_string());
            self.text.insert_char(line_end.to_idx(&self.text).0, '\n');
            let sel = &mut self.selection.selections[indents[i].0];
            sel.cursor = *line_end;
            for fixing_i in 0..=i {
                let fixing_sel = &mut self.selection.selections[indents[fixing_i].0];
                fixing_sel.cursor = fixing_sel
                    .cursor
                    .forward(1 + indent.len_chars(), &self.text);
                fixing_sel.anchor = fixing_sel
                    .anchor
                    .forward(1 + indent.len_chars(), &self.text);
                *fixing_sel = fixing_sel.collapsed();
            }
        }
    }

    pub fn delete(&mut self) -> Vec<Rope> {
        let res = self.map_each_enumerated_selection_mut(|i, sel, text| {
            let range = sel
                .aligned(text)
                .self_or_direction_marker(text)
                .sorted_range_usize();
            let yanked = text.slice(range.clone()).into();
            *sel = sel.collapsed();
            (yanked, i, range)
        });
        let mut removal_points = vec![];
        let mut yanked = vec![];

        for (y, i, r) in res.into_iter() {
            removal_points.push((i, r));
            yanked.push(y);
        }

        self.remove_ranges(removal_points);

        yanked
    }

    pub fn yank(&mut self) -> Vec<Rope> {
        self.map_each_selection_mut(|sel, text| {
            let range = sel.aligned(text).sorted_range_usize();
            text.slice(range).into()
        })
    }

    pub fn paste(&mut self, yanked: &[Rope]) {
        let mut insertion_points = self.map_each_enumerated_selection(|i, sel, text| {
            (i, sel.cursor.trim_column_to_buf(text).to_idx(text))
        });
        insertion_points.sort_by_key(|&(_, idx)| idx);
        insertion_points.reverse();

        // we insert from the back, fixing idx past the insertion every time
        // this is O(n^2) while it could be O(n)
        for (i, (_, idx)) in insertion_points.iter().enumerate() {
            if let Some(to_yank) = yanked.get(i) {
                for chunk in to_yank.chunks() {
                    self.text.insert(idx.0, chunk);
                }
                {
                    let fixing_sel = &mut self.selection.selections[insertion_points[i].0];
                    if fixing_sel.aligned(&self.text).is_forward() {
                        fixing_sel.anchor = fixing_sel.cursor;
                        fixing_sel.cursor =
                            fixing_sel.cursor.forward(to_yank.len_chars(), &self.text);
                    } else {
                        fixing_sel.anchor =
                            fixing_sel.cursor.forward(to_yank.len_chars(), &self.text);
                    }
                }
                for fixing_i in 0..i {
                    let fixing_sel = &mut self.selection.selections[insertion_points[fixing_i].0];
                    if *idx
                        <= fixing_sel
                            .cursor
                            .trim_column_to_buf(&self.text)
                            .to_idx(&self.text)
                    {
                        fixing_sel.cursor =
                            fixing_sel.cursor.forward(to_yank.len_chars(), &self.text);
                    }
                    if *idx
                        <= fixing_sel
                            .anchor
                            .trim_column_to_buf(&self.text)
                            .to_idx(&self.text)
                    {
                        fixing_sel.anchor =
                            fixing_sel.anchor.forward(to_yank.len_chars(), &self.text);
                    }
                }
            }
        }
    }

    pub fn paste_extend(&mut self, yanked: &[Rope]) {
        let mut insertion_points = self.map_each_enumerated_selection(|i, sel, text| {
            (i, sel.cursor.trim_column_to_buf(text).to_idx(text))
        });
        insertion_points.sort_by_key(|&(_, idx)| idx);
        insertion_points.reverse();

        // we insert from the back, fixing idx past the insertion every time
        // this is O(n^2) while it could be O(n)
        for (i, (_, idx)) in insertion_points.iter().enumerate() {
            if let Some(to_yank) = yanked.get(i) {
                for chunk in to_yank.chunks() {
                    self.text.insert(idx.0, chunk);
                }
                {
                    let fixing_sel = &mut self.selection.selections[insertion_points[i].0];
                    if fixing_sel.aligned(&self.text).is_forward() {
                        fixing_sel.cursor =
                            fixing_sel.cursor.forward(to_yank.len_chars(), &self.text);
                    } else {
                        fixing_sel.anchor =
                            fixing_sel.anchor.forward(to_yank.len_chars(), &self.text);
                    }
                }
                for fixing_i in 0..i {
                    let fixing_sel = &mut self.selection.selections[insertion_points[fixing_i].0];
                    if *idx
                        <= fixing_sel
                            .cursor
                            .trim_column_to_buf(&self.text)
                            .to_idx(&self.text)
                    {
                        fixing_sel.cursor =
                            fixing_sel.cursor.forward(to_yank.len_chars(), &self.text);
                    }
                    if *idx
                        <= fixing_sel
                            .anchor
                            .trim_column_to_buf(&self.text)
                            .to_idx(&self.text)
                    {
                        fixing_sel.anchor =
                            fixing_sel.anchor.forward(to_yank.len_chars(), &self.text);
                    }
                }
            }
        }
    }

    /// Remove text at given ranges
    ///
    /// `removal_points` contains list of `(selection_index, range)`,
    fn remove_ranges(&mut self, mut removal_points: Vec<(usize, std::ops::Range<usize>)>) {
        removal_points.sort_by_key(|&(_, ref range)| range.start);
        removal_points.reverse();

        // we remove from the back, fixing idx past the removal every time
        // this is O(n^2) while it could be O(n)
        for (_, (_, range)) in removal_points.iter().enumerate() {
            self.sub_to_every_selection_after(Idx(range.start), range.len());
            // remove has to be after fixes, otherwise to_idx conversion
            // will use the new buffer content, which will give wrong results
            self.text.remove(range.clone());
        }
    }

    pub fn backspace(&mut self) {
        let removal_points = self.map_each_enumerated_selection_mut(|i, sel, text| {
            let sel_aligned = sel.aligned(text);
            let range = (sel_aligned.cursor.0 - 1)..sel_aligned.cursor.0;
            *sel = sel.collapsed();

            (i, range)
        });

        self.remove_ranges(removal_points);
    }

    fn add_to_every_selection_after(&mut self, idx: Idx, offset: usize) {
        self.map_each_selection_mut(|sel, text| {
            let cursor_idx = sel.cursor.to_idx(text);
            let anchor_idx = sel.cursor.to_idx(text);

            if idx <= cursor_idx {
                sel.cursor = Idx(cursor_idx.0.saturating_add(offset)).to_coord(text);
            }
            if idx <= anchor_idx {
                sel.anchor = Idx(anchor_idx.0.saturating_add(offset)).to_coord(text);
            }
        });
    }

    fn sub_to_every_selection_after(&mut self, idx: Idx, offset: usize) {
        self.map_each_selection_mut(|sel, text| {
            let cursor_idx = sel.cursor.to_idx(text);
            let anchor_idx = sel.anchor.to_idx(text);
            if idx < cursor_idx {
                sel.cursor = Idx(cursor_idx.0.saturating_sub(offset)).to_coord(text);
            }
            if idx < anchor_idx {
                sel.anchor = Idx(anchor_idx.0.saturating_sub(offset)).to_coord(text);
            }
        });
    }

    pub fn move_cursor<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> Coord,
    {
        self.map_each_selection_mut(|sel, text| {
            let new_cursor = f(sel.cursor, text);
            sel.anchor = sel.cursor;
            sel.cursor = new_cursor;
        });
    }

    pub fn move_cursor_2<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> (Coord, Coord),
    {
        self.map_each_selection_mut(|sel, text| {
            let (new_anchor, new_cursor) = f(sel.cursor, text);
            sel.anchor = new_anchor;
            sel.cursor = new_cursor;
        });
    }

    pub fn extend_cursor<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> Coord,
    {
        self.map_each_selection_mut(|sel, text| {
            sel.cursor = f(sel.cursor, text);
        });
    }

    pub fn extend_cursor_2<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> (Coord, Coord),
    {
        self.map_each_selection_mut(|sel, text| {
            let (_new_anchor, new_cursor) = f(sel.cursor, text);
            sel.cursor = new_cursor;
        });
    }

    pub fn change_selection<F>(&mut self, f: F)
    where
        F: Fn(Coord, Coord, &Rope) -> (Coord, Coord),
    {
        self.map_each_selection_mut(|sel, text| {
            let (new_cursor, new_anchor) = f(sel.cursor, sel.anchor, text);
            sel.anchor = new_anchor;
            sel.cursor = new_cursor;
        });
    }

    pub fn move_cursor_backward(&mut self, n: usize) {
        self.move_cursor(|coord, text| coord.backward(n, text));
    }

    pub fn move_cursor_forward(&mut self, n: usize) {
        self.move_cursor(|coord, text| coord.forward(n, text));
    }

    pub fn move_cursor_down(&mut self, n: usize) {
        self.move_cursor(|coord, text| coord.down_unaligned(n, text));
    }

    pub fn move_cursor_up(&mut self, n: usize) {
        self.move_cursor(|coord, text| coord.up_unaligned(n, text));
    }

    pub fn extend_cursor_backward(&mut self, n: usize) {
        self.extend_cursor(|coord, text| coord.backward(n, text));
    }

    pub fn extend_cursor_forward(&mut self, n: usize) {
        self.extend_cursor(|coord, text| coord.forward(n, text));
    }

    pub fn extend_cursor_down(&mut self, n: usize) {
        self.extend_cursor(|coord, text| coord.down_unaligned(n, text));
    }

    pub fn extend_cursor_up(&mut self, n: usize) {
        self.extend_cursor(|coord, text| coord.up_unaligned(n, text));
    }

    pub fn move_cursor_forward_word(&mut self) {
        self.move_cursor_2(Coord::forward_word)
    }

    pub fn move_cursor_backward_word(&mut self) {
        self.move_cursor_2(Coord::backward_word)
    }

    pub fn cursor_pos(&self) -> Coord {
        self.selection.selections[0]
            .cursor
            .trim_column_to_buf(&self.text)
    }

    pub fn move_line(&mut self) {
        self.change_selection(|cursor, _anchor, text| {
            (
                cursor.forward_past_line_end(text),
                cursor.backward_to_line_start(text),
            )
        });
    }

    pub fn extend_line(&mut self) {
        self.change_selection(|cursor, anchor, text| {
            let anchor = min(cursor, anchor);

            (
                cursor.forward_past_line_end(text),
                if anchor.column == 0 {
                    anchor
                } else {
                    anchor.backward_to_line_start(text)
                },
            )
        });
    }

    pub fn select_all(&mut self) {
        self.selection.selections = vec![SelectionUnaligned::from_selection(
            if self.selection.selections[self.selection.primary]
                .aligned(&self.text)
                .is_forward()
            {
                Selection {
                    anchor: Idx(0),
                    cursor: Idx(self.text.len_chars()),
                    was_forward: true,
                }
            } else {
                Selection {
                    cursor: Idx(0),
                    anchor: Idx(self.text.len_chars()),
                    was_forward: false,
                }
            },
            &self.text,
        )];
    }

    pub fn collapse(&mut self) {
        if self.selection.selections.len() > 1 {
            self.selection.selections = vec![self.selection.selections[self.selection.primary]];
        } else {
            self.selection.selections[self.selection.primary] =
                self.selection.selections[self.selection.primary].collapsed();
        }
    }

    pub fn to_visual(&self, coord: Coord) -> Coord {
        let line = self.text.line(coord.line);
        let v_col = line.slice(..coord.column).chars().fold(0, |v_col, ch| {
            if ch == '\t' {
                v_col + distance_to_next_tabstop(v_col, self.tabstop)
            } else {
                v_col + 1
            }
        });

        Coord {
            line: coord.line,
            column: v_col,
        }
    }

    pub fn increase_indent(&self, _times: usize) {
        let _affected_lines = self.selection.to_lines();
        unimplemented!();
    }

    pub fn decrease_indent(&self, _times: usize) {
        let _affected_lines = self.selection.to_lines();
        unimplemented!();
    }
}
