#![allow(dead_code)]
use crate::{coord::*, idx::*, prelude::*, selection::*, util::char};
use ropey::Rope;
use std::cell::RefCell;
use std::cmp::min;
use std::collections::BTreeSet;
use std::path::PathBuf;

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

pub fn distance_to_prev_tabstop(visual_column: usize, tabstop: usize) -> usize {
    let tabstop = visual_column.saturating_sub(1) / tabstop * tabstop;
    visual_column - tabstop
}

#[test]
fn distance_to_next_tabstop_test() {
    for (v_col, expected) in &[(0, 4), (1, 3), (2, 2), (3, 1), (4, 4)] {
        assert_eq!(distance_to_next_tabstop(*v_col, 4), *expected);
    }
}

#[test]
fn distance_to_prev_tabstop_test() {
    for (v_col, expected) in &[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 1)] {
        assert_eq!(distance_to_prev_tabstop(*v_col, 4), *expected);
    }
}

fn is_line_prefix_increasing_ident_level(start: Idx, end: Idx, text: &Rope) -> bool {
    let mut level = 0isize;

    for ch in start.range_to(end).slice(text).chars() {
        if char::is_opening_indent(ch) {
            level += 1;
        } else if char::is_closing_indent(ch) {
            level -= 1;
        }
    }

    level > 0
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
            let to = std::cmp::max(from, to.backward(text)).to_coord(text);
            let from = from.to_coord(text);
            for line in from.line..=to.line {
                lines.insert(line);
            }
        }
        lines
    }

    pub fn fix_on_insert(&mut self, idx: Idx, len: usize) {
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

    pub fn fix_on_delete(&mut self, idx: Idx, len: usize, text: &Rope) {
        for i in 0..self.selections.len() {
            let sel = &mut self.selections[i];
            let cursor_idx = &mut sel.cursor;
            let anchor_idx = &mut sel.anchor;
            if idx < cursor_idx.backward_n(len, text) {
                *cursor_idx = Idx(cursor_idx.0.saturating_sub(len));
            } else if idx < *cursor_idx {
                *cursor_idx = Idx(cursor_idx.0.saturating_sub(cursor_idx.0 - idx.0));
            }
            if idx < anchor_idx.backward_n(len, text) {
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

    pub path: Option<PathBuf>,

    pub tabstop: usize,
    pub expand_tabs: bool,

    pub view_line_offset: RefCell<usize>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            text: Rope::default(),
            tabstop: 4,
            selection: default(),
            expand_tabs: true,
            path: None,
            view_line_offset: RefCell::new(0),
        }
    }
}

impl Buffer {
    pub fn from_text(text: Rope) -> Self {
        Self { text, ..default() }
    }

    pub fn lines(&self) -> usize {
        self.text.len_lines()
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

    fn map_each_selection<F, R>(&self, mut f: F) -> Vec<R>
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
            sel.is_empty()
                && sel
                    .aligned(&self.text)
                    .is_idx_inside_direction_marker(idx, &self.text)
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

    pub fn insert_tab(&mut self) {
        self.selection.clear_cursor_column();

        if self.expand_tabs {
            let mut insertions = self.map_each_selection(|sel, text| {
                let v_col = self.to_visual(sel.cursor.to_coord(text)).column;

                (sel.cursor, distance_to_next_tabstop(v_col, self.tabstop))
            });

            insertions.sort_by_key(|insertion| insertion.0);
            insertions.reverse();

            for (idx, n) in insertions {
                self.selection.collapse();
                self.selection.sort();
                self.selection.fix_on_insert(idx, n);
                self.text.insert(idx.0, &" ".repeat(n));
            }
        } else {
            self.insert_char('\t');
        }
    }

    pub fn insert(&mut self, s: &str) {
        self.selection.clear_cursor_column();

        let mut insertion_points = self.map_each_selection_mut(|sel, _text| sel.cursor);
        insertion_points.sort();
        insertion_points.reverse();

        self.selection.collapse();
        self.selection.sort();

        for idx in insertion_points {
            if !s.is_empty() {
                self.selection.fix_on_insert(idx, s.len());
                self.text.insert(idx.0, s);
            }
        }
    }

    pub fn insert_enter(&mut self) {
        self.open_impl(true);
    }

    pub fn open(&mut self) {
        self.open_impl(false);
    }

    fn open_impl(&mut self, was_enter: bool) {
        self.selection.clear_cursor_column();
        let mut indents = self.map_each_enumerated_selection(|i, sel, text| {
            let line_begining = sel.cursor.backward_to_line_start(text);
            let indent_end = sel.cursor.before_first_non_whitespace(text);
            let indent: Rope = line_begining.range_to(indent_end).slice(text).into();
            let insert_idx = if was_enter {
                sel.cursor
            } else {
                sel.cursor.forward_to_line_end(text)
            };
            let increase_indent =
                is_line_prefix_increasing_ident_level(line_begining, insert_idx, text);
            (i, indent, insert_idx, increase_indent)
        });
        indents.sort_by_key(|&(_, _, insert_idx, _)| insert_idx);
        indents.reverse();

        self.selection.collapse();
        for (i, (_, indent, insert_idx, increase_indent)) in indents.iter().enumerate() {
            let mut inserted_len = 0;
            self.text.insert(insert_idx.0, &indent.to_string());
            inserted_len += indent.len_chars();
            if *increase_indent {
                let indent_text = &self.indent_text(1);
                self.text.insert(insert_idx.0, &indent_text);
                inserted_len += indent_text.len();
            }
            self.text.insert_char(insert_idx.0, '\n');
            inserted_len += 1;

            self.selection.fix_on_insert(*insert_idx, inserted_len);
            let sel = &mut self.selection.selections[indents[i].0];
            sel.cursor = insert_idx.forward_n(inserted_len, &self.text);
            *sel = sel.collapsed();
        }
    }

    pub fn delete(&mut self) -> Vec<Rope> {
        self.selection.clear_cursor_column();
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
                self.selection.fix_on_insert(*idx, to_yank.len_chars());
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
                self.selection.fix_on_insert(*idx, to_yank.len_chars());
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

        for range in removal_points {
            self.selection
                .fix_on_delete(Idx(range.start), range.len(), &self.text);
            self.text.remove(range.clone());
        }
    }

    pub fn backspace_one(&mut self) {
        self.selection.clear_cursor_column();
        let removal_points = self.map_each_enumerated_selection_mut(|_, sel, text| {
            let sel_aligned = sel.aligned(text);
            let range = (sel_aligned.cursor.0 - 1)..sel_aligned.cursor.0;
            *sel = sel.collapsed();

            range
        });

        self.remove_ranges(removal_points);
    }

    pub fn backspace(&mut self) {
        self.selection.clear_cursor_column();
        if self.expand_tabs {
            let mut removal = self.map_each_selection(|sel, text| {
                let v_col = self.to_visual(sel.cursor.to_coord(text)).column;

                (
                    sel.cursor,
                    if v_col == 0 {
                        1
                    } else if sel.cursor == sel.cursor.before_first_non_whitespace(text) {
                        distance_to_prev_tabstop(v_col, self.tabstop)
                    } else {
                        1
                    },
                )
            });

            removal.sort_by_key(|r| r.0);
            removal.reverse();

            self.selection.collapse();
            self.selection.sort();
            for (idx, n) in removal {
                let start = idx.backward_n(n, &self.text);
                self.selection
                    .fix_on_delete(start, idx.0 - start.0, &self.text);
                self.text.remove(start.0..idx.0);
            }
        } else {
            self.backspace_one();
        }
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
        self.move_cursor(|idx, text| idx.backward_n(n, text));
    }

    pub fn move_cursor_forward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.move_cursor(|idx, text| idx.forward_n(n, text));
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
        self.extend_cursor(|idx, text| idx.backward_n(n, text));
    }

    pub fn extend_cursor_forward(&mut self, n: usize) {
        self.selection.clear_cursor_column();
        self.extend_cursor(|idx, text| idx.forward_n(n, text));
    }

    pub fn move_cursor_forward_word(&mut self) {
        self.selection.clear_cursor_column();
        self.move_cursor_2(Idx::forward_word)
    }

    pub fn move_cursor_backward_word(&mut self) {
        self.selection.clear_cursor_column();
        self.move_cursor_2(Idx::backward_word)
    }

    pub fn cursor_coord(&self) -> Coord {
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

    pub fn increase_indent(&mut self, times: usize) {
        let affected_lines = self.selection.to_lines(&self.text);
        let mut insertions: Vec<_> = affected_lines
            .into_iter()
            .map(|line| Coord { line, column: 0 }.to_idx(&self.text))
            .collect();

        insertions.sort_by_key(|insertion| insertion.0);
        insertions.reverse();

        let text = if !self.expand_tabs {
            "\t".to_owned()
        } else {
            " ".repeat(self.tabstop * times)
        };

        for idx in insertions {
            self.selection.fix_on_insert(idx, text.len());
            self.text.insert(idx.0, &text);
        }
    }

    fn indent_text(&self, times: usize) -> String {
        if !self.expand_tabs {
            "\t".to_owned()
        } else {
            " ".repeat(self.tabstop * times)
        }
    }

    pub fn decrease_indent(&mut self, times: usize) {
        let affected_lines = self.selection.to_lines(&self.text);

        let mut removals: Vec<_> = affected_lines
            .into_iter()
            .map(|line| Coord { line, column: 0 }.to_idx(&self.text))
            .collect();

        removals.sort_by_key(|insertion| insertion.0);
        removals.reverse();

        let indent_text = self.indent_text(times);

        for idx in removals {
            let range = idx.range_to(idx.forward_n(indent_text.len(), &self.text));
            if range.len() < indent_text.len() {
                continue;
            }
            let existing = range.slice(&self.text);
            if existing == indent_text {
                self.selection
                    .fix_on_delete(idx, indent_text.len(), &self.text);
                range.remove_from(&mut self.text);
            }
        }
    }
}
