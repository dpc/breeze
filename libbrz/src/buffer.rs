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
    pub selections: Vec<Selection>,
    /// If this is non-empty, it contains column position, recorded
    /// to preserve a column during up/down moves between lines
    /// that might be shorter
    pub cursor_column: Vec<usize>,
}

impl Default for SelectionSet {
    fn default() -> Self {
        let sel = Selection::default();
        Self {
            selections: vec![sel],
            primary: 0,
            cursor_column: vec![],
        }
    }
}

impl SelectionSet {
    pub fn to_lines(&self, text: &Rope) -> BTreeSet<usize> {
        let mut lines = BTreeSet::new();
        for s in &self.selections {
            let (from, to) = s.sorted_pair();
            let from = from.to_coord(text);
            let to = to.to_coord(text);
            for line in from.line..=to.line {
                lines.insert(line);
            }
        }
        lines
    }

    pub fn fix_before_insert(&mut self, idx: Idx, len: usize) {
        for i in 0..self.selections.len() {
            let sel = &mut self.selections[i];
            let cursor_idx = sel.cursor;
            let anchor_idx = sel.anchor;
            if sel.is_forward() {
                if idx <= cursor_idx {
                    sel.cursor = Idx(cursor_idx.0.saturating_add(len));
                }
                if idx < anchor_idx {
                    sel.anchor = Idx(anchor_idx.0.saturating_add(len));
                }
            } else {
                if idx < cursor_idx {
                    sel.cursor = Idx(cursor_idx.0.saturating_add(len));
                }
                if idx <= anchor_idx {
                    sel.anchor = Idx(anchor_idx.0.saturating_add(len));
                }
            }
        }
    }

    pub fn fix_before_delete(&mut self, idx: Idx, len: usize) {
        for i in 0..self.selections.len() {
            let sel = &mut self.selections[i];
            let cursor_idx = &mut sel.cursor;
            let anchor_idx = &mut sel.anchor;
            if idx < cursor_idx.backward(len) {
                *cursor_idx = Idx(cursor_idx.0.saturating_sub(len));
            } else if idx < *cursor_idx {
                *cursor_idx = Idx(cursor_idx.0.saturating_sub(cursor_idx.0 - idx.0));
            }
            if idx < anchor_idx.backward(len) {
                *anchor_idx = Idx(anchor_idx.0.saturating_sub(len));
            } else if idx < *anchor_idx {
                *anchor_idx = Idx(anchor_idx.0.saturating_sub(anchor_idx.0 - idx.0));
            }
        }
    }

    pub fn maybe_save_cursor_column(&mut self, text: &Rope) {
        if self.cursor_column.is_empty() {
            self.cursor_column = self
                .selections
                .iter()
                .map(|s| s.cursor.to_coord(text).column)
                .collect();
        }
    }

    pub fn clear_cursor_column(&mut self) {
        self.cursor_column.clear();
    }

    pub fn collapse(&mut self) {
        for i in 0..self.selections.len() {
            let sel = &mut self.selections[i];
            *sel = sel.collapsed();
        }
    }

    pub fn sort(&mut self) {
        for i in 0..self.selections.len() {
            let sel = &mut self.selections[i];
            *sel = sel.sorted().update_last_direction();
        }
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
        F: FnMut(&Selection, &Rope) -> R,
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
        F: FnMut(&mut Selection, &mut Rope) -> R,
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
        F: FnMut(usize, &Selection, &Rope) -> R,
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
        F: FnMut(usize, &mut Selection, &mut Rope) -> R,
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
            sel.is_empty() && sel.aligned(&self.text).is_idx_inside_direction_marker(idx)
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
        let mut insertion_points = self.map_each_selection_mut(|sel, _text| sel.cursor);
        insertion_points.sort();
        insertion_points.reverse();

        for idx in insertion_points {
            self.selection.collapse();
            self.selection.sort();
            self.selection.fix_before_insert(idx, s.len());
            self.text.insert(idx.0, s);
        }
    }

    pub fn open(&mut self) {
        let mut indents = self.map_each_enumerated_selection(|i, sel, text| {
            let line_begining = sel.cursor.backward_to_line_start(text).0;
            let indent_end = sel.cursor.before_first_non_whitespace(text).0;
            let indent: Rope = text.slice(line_begining..indent_end).into();
            let line_end = sel.cursor.forward_to_line_end(text);
            (i, indent, line_end)
        });
        indents.sort_by_key(|&(_, _, line_end)| line_end);
        indents.reverse();

        for (i, (_, indent, line_end)) in indents.iter().enumerate() {
            self.text.insert(line_end.0, &indent.to_string());
            self.text.insert_char(line_end.0, '\n');
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
                *fixing_sel = fixing_sel.collapsed().sorted();
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

        for (y, _, r) in res.into_iter() {
            removal_points.push(r);
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
        let mut insertion_points = self.map_each_selection_mut(|sel, _text| (sel.cursor));
        insertion_points.sort();
        insertion_points.reverse();

        for (i, idx) in insertion_points.iter().enumerate() {
            self.selection.collapse();
            if let Some(to_yank) = yanked.get(i) {
                self.selection.fix_before_insert(*idx, to_yank.len_chars());
            }
        }

        for (i, idx) in insertion_points.iter().enumerate() {
            if let Some(to_yank) = yanked.get(i) {
                for chunk in to_yank.chunks() {
                    self.text.insert(idx.0, chunk);
                }
            }
        }
    }

    pub fn paste_extend(&mut self, yanked: &[Rope]) {
        let mut insertion_points = self.map_each_enumerated_selection(|_i, sel, _text| sel.cursor);
        insertion_points.sort();
        insertion_points.reverse();

        for (i, idx) in insertion_points.iter().enumerate() {
            if let Some(to_yank) = yanked.get(i) {
                self.selection.fix_before_insert(*idx, to_yank.len_chars());
            }
        }

        for (i, idx) in insertion_points.iter().enumerate() {
            if let Some(to_yank) = yanked.get(i) {
                for chunk in to_yank.chunks() {
                    self.text.insert(idx.0, chunk);
                }
            }
        }
    }

    /// Remove text at given ranges
    ///
    /// `removal_points` contains list of `(selection_index, range)`,
    fn remove_ranges(&mut self, mut removal_points: Vec<std::ops::Range<usize>>) {
        removal_points.sort_by(|a, b| a.start.cmp(&b.start));
        removal_points.reverse();

        for range in &removal_points {
            self.selection
                .fix_before_delete(Idx(range.start), range.len());
        }

        // remove has to be after fixes, otherwise to_idx conversion
        // will use the new buffer content, which will give wrong results
        for range in &removal_points {
            self.text.remove(range.clone());
        }
    }

    pub fn backspace(&mut self) {
        let removal_points = self.map_each_enumerated_selection_mut(|_, sel, text| {
            let sel_aligned = sel.aligned(text);
            let range = (sel_aligned.cursor.0 - 1)..sel_aligned.cursor.0;
            *sel = sel.collapsed();

            range
        });

        self.remove_ranges(removal_points);
    }

    pub fn move_cursor<F>(&mut self, f: F)
    where
        F: Fn(Idx, &Rope) -> Idx,
    {
        self.map_each_selection_mut(|sel, text| {
            let new_cursor = f(sel.cursor, text);
            sel.anchor = sel.cursor;
            sel.cursor = new_cursor;
        });
    }

    pub fn move_cursor_with_column<F>(&mut self, f: F)
    where
        F: Fn(Idx, Option<usize>, &Rope) -> Idx,
    {
        let selection = self.selection.clone();
        self.map_each_enumerated_selection_mut(|i, sel, text| {
            let column = selection.cursor_column.get(i).cloned();
            let new_cursor = f(sel.cursor, column, text);
            sel.anchor = sel.cursor;
            sel.cursor = new_cursor;
        });
    }

    pub fn move_cursor_2<F>(&mut self, f: F)
    where
        F: Fn(Idx, &Rope) -> (Idx, Idx),
    {
        self.map_each_selection_mut(|sel, text| {
            let (new_anchor, new_cursor) = f(sel.cursor, text);
            sel.anchor = new_anchor;
            sel.cursor = new_cursor;
        });
    }

    pub fn extend_cursor<F>(&mut self, f: F)
    where
        F: Fn(Idx, &Rope) -> Idx,
    {
        self.map_each_selection_mut(|sel, text| {
            sel.cursor = f(sel.cursor, text);
        });
    }

    pub fn extend_cursor_with_column<F>(&mut self, f: F)
    where
        F: Fn(Idx, Option<usize>, &Rope) -> Idx,
    {
        let selection = self.selection.clone();
        self.map_each_enumerated_selection_mut(|i, sel, text| {
            let column = selection.cursor_column.get(i).cloned();
            sel.cursor = f(sel.cursor, column, text);
        });
    }

    pub fn extend_cursor_2<F>(&mut self, f: F)
    where
        F: Fn(Idx, &Rope) -> (Idx, Idx),
    {
        self.map_each_selection_mut(|sel, text| {
            let (_new_anchor, new_cursor) = f(sel.cursor, text);
            sel.cursor = new_cursor;
        });
    }

    pub fn change_selection<F>(&mut self, f: F)
    where
        F: Fn(Idx, Idx, &Rope) -> (Idx, Idx),
    {
        self.map_each_selection_mut(|sel, text| {
            let (new_cursor, new_anchor) = f(sel.cursor, sel.anchor, text);
            sel.anchor = new_anchor;
            sel.cursor = new_cursor;
        });
    }

    pub fn move_cursor_coord<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> Coord,
    {
        self.map_each_selection_mut(|sel, text| {
            let new_cursor = f(sel.cursor.to_coord(text), text);
            sel.anchor = sel.cursor;
            sel.cursor = new_cursor.to_idx(text);
        });
    }

    pub fn extend_cursor_coord<F>(&mut self, f: F)
    where
        F: Fn(Coord, &Rope) -> Coord,
    {
        self.map_each_selection_mut(|sel, text| {
            sel.cursor = f(sel.cursor.to_coord(text), text).to_idx(text);
        });
    }
    pub fn move_cursor_backward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.move_cursor(|idx, _text| idx.backward(n));
    }

    pub fn move_cursor_forward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.move_cursor(|idx, text| idx.forward(n, text));
    }

    pub fn move_cursor_down(&mut self, n: usize) {
        self.selection.maybe_save_cursor_column(&self.text);

        self.move_cursor_with_column(|idx, column, text| idx.down_unaligned(n, column, text));
    }

    pub fn move_cursor_up(&mut self, n: usize) {
        self.selection.maybe_save_cursor_column(&self.text);
        self.move_cursor_with_column(|idx, column, text| idx.up_unaligned(n, column, text));
    }

    pub fn extend_cursor_down(&mut self, n: usize) {
        self.selection.maybe_save_cursor_column(&self.text);
        self.extend_cursor_with_column(|idx, column, text| idx.down_unaligned(n, column, text));
    }

    pub fn extend_cursor_up(&mut self, n: usize) {
        self.selection.maybe_save_cursor_column(&self.text);
        self.extend_cursor_with_column(|idx, column, text| idx.up_unaligned(n, column, text));
    }

    pub fn extend_cursor_backward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.extend_cursor(|idx, _text| idx.backward(n));
    }

    pub fn extend_cursor_forward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.extend_cursor(|idx, text| idx.forward(n, text));
    }

    pub fn move_cursor_forward_word(&mut self) {
        self.selection.clear_cursor_column();
        self.move_cursor_2(Idx::forward_word)
    }

    pub fn move_cursor_backward_word(&mut self) {
        self.selection.clear_cursor_column();
        self.move_cursor_2(Idx::backward_word)
    }

    pub fn cursor_pos(&self) -> Coord {
        self.selection.selections[0].cursor.to_coord(&self.text)
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
                if anchor.to_coord(text).column == 0 {
                    anchor
                } else {
                    anchor.backward_to_line_start(text)
                },
            )
        });
    }

    pub fn select_all(&mut self) {
        self.selection.selections = vec![if self.selection.selections[self.selection.primary]
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
        }];
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
        let _affected_lines = self.selection.to_lines(&self.text);
        unimplemented!();
    }

    pub fn decrease_indent(&self, _times: usize) {
        let _affected_lines = self.selection.to_lines(&self.text);
        unimplemented!();
    }
}
