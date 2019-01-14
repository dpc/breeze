#![allow(dead_code)]

mod prelude;

use self::prelude::*;

use std::sync::Arc;

use std::cmp::min;
use std::io::Write;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;

use ropey::Rope;

fn sub_rope(text: &Rope, start: usize, end: usize) -> Rope {
    let mut sub = text.clone();

    eprintln!("{} {} {}", line!(), start, end);
    assert!(start <= end);

    if end < text.len_chars() {
        sub.remove(end..text.len_chars());
    }
    sub.remove(..start);

    sub
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
/// Coordinate where the column can exceed the line length
struct CoordUnaligned {
    line: usize,
    column: usize,
}

impl From<Coord> for CoordUnaligned {
    fn from(coord: Coord) -> Self {
        Self {
            line: coord.line,
            column: coord.column,
        }
    }
}

impl CoordUnaligned {
    fn map_as_coord<F>(self, text: &Rope, f: F) -> Self
    where
        F: FnOnce(Coord, &Rope) -> Coord,
    {
        f(self.align(text), text).into()
    }

    fn forward(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::forward)
    }

    fn forward_n(self, n: usize, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_n(n, text))
    }

    fn forward_to_line_end(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_to_line_end(text))
    }

    fn forward_past_line_end(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.forward_past_line_end(text))
    }

    fn backward(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::backward)
    }

    fn backward_word(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::backward_word)
    }

    fn backward_to_line_start(self, text: &Rope) -> Self {
        self.map_as_coord(text, |coord, text| coord.backward_to_line_start(text))
    }

    fn forward_word(self, text: &Rope) -> Self {
        self.map_as_coord(text, Coord::forward_word)
    }

    fn up_unaligned(self, _text: &Rope) -> Self {
        Self {
            line: self.line.saturating_sub(1),
            column: self.column,
        }
    }

    fn down_unaligned(self, text: &Rope) -> Self {
        let lines = text.len_lines();
        Self {
            line: if self.line == lines || self.line + 1 == lines {
                self.line
            } else {
                self.line + 1
            },
            column: self.column,
        }
    }
}

impl CoordUnaligned {
    /// Align to buffer
    ///
    /// Column in the `Coord` can actually exeed the actual column,
    /// which is useful eg. for consecutive up and down movements
    fn align(self, text: &Rope) -> Coord {
        let line = text.line(self.line);
        let line_len = line.len_chars();
        let trimed_column = if line_len == 0 {
            0
        } else if self.line + 1 == text.len_lines() {
            std::cmp::min(self.column, line_len)
        } else {
            std::cmp::min(self.column, line_len - 1)
        };

        Coord {
            line: self.line,
            column: trimed_column,
        }
    }

    fn trim(self, text: &Rope) -> Self {
        Self {
            column: self.column,
            line: min(self.line, text.len_lines().saturating_sub(1)),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
/// Coordinate where the row is known to be within the line length
///
/// Note: This is within the buffer this `Coord` was created to work
/// in.
struct Coord {
    line: usize,
    column: usize,
}

impl Coord {
    fn map_as_idx<F>(self, text: &Rope, f: F) -> Self
    where
        F: FnOnce(Idx) -> Idx,
    {
        Self::from_idx(f(self.to_idx(text)), text)
    }

    fn to_idx(self, text: &Rope) -> Idx {
        (text.line_to_char(self.line) + self.column).into()
    }

    fn from_idx(idx: Idx, text: &Rope) -> Self {
        let line = text.char_to_line(idx.0);
        let line_start_pos = text.line_to_char(line);
        let column = idx.0 - line_start_pos;

        Self { line, column }
    }

    fn forward(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward(text), text)
    }

    fn forward_n(self, n: usize, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_n(n, text), text)
    }

    fn forward_to_line_end(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_to_line_end(text), text)
    }

    fn forward_past_line_end(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).forward_past_line_end(text), text)
    }

    fn backward_to_line_start(self, text: &Rope) -> Self {
        Self::from_idx(self.to_idx(text).backward_to_line_start(text), text)
    }

    fn backward(self, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.backward(text))
    }

    fn forward_word(self, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.forward_word(text))
    }

    fn backward_word(self, text: &Rope) -> Self {
        self.map_as_idx(text, |idx| idx.backward_word(text))
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Idx(usize);

impl Idx {
    fn backward(self, _text: &Rope) -> Self {
        Idx(self.0.saturating_sub(1))
    }

    fn forward(self, text: &Rope) -> Self {
        self.forward_n(1, text)
    }

    fn forward_n(self, n: usize, text: &Rope) -> Self {
        Idx(std::cmp::min(self.0.saturating_add(n), text.len_chars()))
    }

    fn to_coord(self, text: &Rope) -> Coord {
        Coord::from_idx(self, text)
    }

    fn backward_word(self, text: &Rope) -> Idx {
        let mut cur = std::cmp::min(self.0, text.len_chars().saturating_sub(1));
        loop {
            if cur == 0 {
                break;
            }
            let prev = cur.saturating_sub(1);
            let ch = text.char(prev);
            if prev > 0 && !ch.is_alphanumeric() {
                cur -= 1;
            } else {
                break;
            }
        }
        loop {
            if cur == 0 {
                break;
            }
            let prev = cur.saturating_sub(1);
            let ch = text.char(prev);
            if ch.is_alphanumeric() && ch != '\n' {
                cur -= 1;
            } else {
                break;
            }
        }
        Idx(cur)
    }

    fn forward_word(self, text: &Rope) -> Idx {
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
        loop {
            if cur == text_len {
                break;
            }
            let ch = text.char(cur);
            if ch.is_alphanumeric() {
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
            if !ch.is_alphanumeric() && ch != '\n' {
                cur += 1;
            } else {
                break;
            }
        }
        Idx(cur)
    }

    fn forward_to_line_end(self, text: &Rope) -> Idx {
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

    fn forward_past_line_end(self, text: &Rope) -> Idx {
        self.forward_to_line_end(text).forward(text)
    }

    fn backward_to_line_start(self, text: &Rope) -> Idx {
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
/// Selection with `CoordUnaligned`
///
/// An ordererd pair of indices in the buffer
#[derive(Default, Debug, Clone, Copy)]
struct SelectionUnaligned {
    anchor: CoordUnaligned,
    cursor: CoordUnaligned,
}

impl SelectionUnaligned {
    fn align(self, text: &Rope) -> Selection {
        Selection {
            anchor: self.anchor.align(text).to_idx(text),
            cursor: self.cursor.align(text).to_idx(text),
        }
    }

    fn trim(self, text: &Rope) -> Self {
        Self {
            anchor: self.anchor.trim(text),
            cursor: self.cursor.trim(text),
        }
    }

    /// Colapse anchor to the cursor
    fn collapsed(self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.cursor,
        }
    }

    fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
/// Selection with coordinates aligned
///
/// As coordinates are aligned, it's OK to keep
/// just the index in the text.
struct Selection {
    anchor: Idx,
    cursor: Idx,
}

impl Selection {
    fn is_idx_inside(self, idx: Idx) -> bool {
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

    fn is_forward(self) -> Option<bool> {
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

    fn sorted(self) -> (Idx, Idx) {
        if self.anchor < self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    fn sorted_range(self) -> std::ops::Range<Idx> {
        let (a, b) = self.sorted();
        a..b
    }

    fn sorted_range_usize(self) -> std::ops::Range<usize> {
        let (a, b) = self.sorted();
        a.into()..b.into()
    }

    /// Colapse anchor to the cursor
    fn collapsed(self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.cursor,
        }
    }

    fn reversed(self) -> Self {
        Self {
            anchor: self.cursor,
            cursor: self.anchor,
        }
    }
}

#[derive(Debug, Clone)]
struct Buffer {
    text: ropey::Rope,
    selections: Vec<SelectionUnaligned>,
    primary_sel_i: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        let sel = SelectionUnaligned::default();
        Self {
            text: Rope::default(),
            selections: vec![sel],
            primary_sel_i: 0,
        }
    }
}

impl Buffer {
    fn for_each_selection_mut<F, R>(&mut self, mut f: F) -> Vec<R>
    where
        F: FnMut(&mut SelectionUnaligned, &mut Rope) -> R,
    {
        let Self {
            ref mut selections,
            ref mut text,
            ..
        } = *self;

        selections.iter_mut().map(|sel| f(sel, text)).collect()
    }

    fn for_each_enumerated_selection_mut<F, R>(&mut self, mut f: F) -> Vec<R>
    where
        F: FnMut(usize, &mut SelectionUnaligned, &mut Rope) -> R,
    {
        let Self {
            ref mut selections,
            ref mut text,
            ..
        } = *self;

        selections
            .iter_mut()
            .enumerate()
            .map(|(i, sel)| f(i, sel, text))
            .collect()
    }

    fn is_idx_selected(&self, idx: Idx) -> bool {
        self.selections
            .iter()
            .any(|sel| sel.align(&self.text).is_idx_inside(idx))
    }

    fn reverse_selections(&mut self) {
        self.for_each_selection_mut(|sel, _text| *sel = sel.reversed());
    }

    fn insert(&mut self, ch: char) {
        self.for_each_selection_mut(move |sel, text| {
            let aligned_cursor = sel.cursor.align(text);
            text.insert_char(aligned_cursor.to_idx(text).into(), ch);

            sel.anchor = sel.cursor;
            sel.cursor = sel.cursor.forward(text);
        });
    }

    fn delete(&mut self) -> Vec<Rope> {
        let yanked = self.for_each_selection_mut(|sel, text| {
            let range = sel.align(text).sorted_range_usize();
            let yanked = sub_rope(text, range.start, range.end);
            text.remove(range);
            *sel = sel.collapsed().trim(text);
            yanked
        });

        self.for_each_selection_mut(|sel, text| {
            *sel = sel.trim(text);
        });

        yanked
    }

    fn yank(&mut self) -> Vec<Rope> {
        self.for_each_selection_mut(|sel, text| {
            let range = sel.align(text).sorted_range_usize();
            sub_rope(text, range.start, range.end)
        })
    }

    fn paste(&mut self, yanked: &[Rope]) {
        self.for_each_enumerated_selection_mut(|i, sel, text| {
            if let Some(to_yank) = yanked.get(i) {
                for chunk in to_yank.chunks() {
                    let aligned_cursor = sel.cursor.align(text);
                    text.insert(aligned_cursor.to_idx(text).into(), chunk);
                    if !sel.align(text).is_forward().unwrap_or(false) {
                        sel.anchor = sel.anchor.forward_n(chunk.len(), text);
                        sel.cursor = sel.cursor.forward_n(chunk.len(), text);
                    }
                }
            }
        });
    }

    fn backspace(&mut self) {
        self.for_each_selection_mut(|sel, text| {
            let sel_aligned = sel.align(text);
            if sel_aligned.cursor == 0usize.into() {
                return;
            }

            match sel_aligned.is_forward() {
                Some(true) => {
                    sel.cursor = sel.cursor.backward(text);
                }
                _ => {
                    sel.cursor = sel.cursor.backward(text);
                    sel.anchor = sel.anchor.backward(text);
                }
            }

            text.remove((sel_aligned.cursor.0 - 1)..sel_aligned.cursor.0);
        });

        self.for_each_selection_mut(|sel, text| {
            *sel = sel.trim(text);
        });
    }

    fn move_cursor<F>(&mut self, f: F)
    where
        F: Fn(CoordUnaligned, &Rope) -> CoordUnaligned,
    {
        self.for_each_selection_mut(|sel, text| {
            let new_cursor = f(sel.cursor, text);
            sel.anchor = sel.cursor;
            sel.cursor = new_cursor;
        });
    }

    fn extend_cursor<F>(&mut self, f: F)
    where
        F: Fn(CoordUnaligned, &Rope) -> CoordUnaligned,
    {
        self.for_each_selection_mut(|sel, text| {
            sel.cursor = f(sel.cursor, text);
        });
    }

    fn change_selection<F>(&mut self, f: F)
    where
        F: Fn(CoordUnaligned, CoordUnaligned, &Rope) -> (CoordUnaligned, CoordUnaligned),
    {
        self.for_each_selection_mut(|sel, text| {
            let (new_cursor, new_anchor) = f(sel.cursor, sel.anchor, text);
            sel.anchor = new_anchor;
            sel.cursor = new_cursor;
        });
    }

    fn move_cursor_backward(&mut self) {
        self.move_cursor(CoordUnaligned::backward);
    }

    fn move_cursor_forward(&mut self) {
        self.move_cursor(CoordUnaligned::forward);
    }

    fn move_cursor_down(&mut self) {
        self.move_cursor(CoordUnaligned::down_unaligned);
    }

    fn move_cursor_up(&mut self) {
        self.move_cursor(CoordUnaligned::up_unaligned);
    }

    fn move_cursor_forward_word(&mut self) {
        self.move_cursor(CoordUnaligned::forward_word)
    }

    fn move_cursor_backward_word(&mut self) {
        self.move_cursor(CoordUnaligned::backward_word)
    }

    fn cursor_pos(&self) -> Coord {
        self.selections[0].cursor.align(&self.text)
    }

    fn move_line(&mut self) {
        self.change_selection(|cursor, _anchor, text| {
            (
                cursor.forward_past_line_end(text),
                cursor.backward_to_line_start(text),
            )
        });
    }

    fn extend_line(&mut self) {
        self.change_selection(|cursor, anchor, text| {
            (
                cursor.forward_past_line_end(text),
                anchor.backward_to_line_start(text),
            )
        });
    }
}

trait Mode {
    fn handle(&self, state: State, key: Key) -> State;
    fn name(&self) -> &str;
}

struct InsertMode;
struct NormalMode;

impl Mode for InsertMode {
    fn name(&self) -> &str {
        "insert"
    }

    fn handle(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Esc => {
                state.modes.pop();
            }
            Key::Char('\n') => {
                state.buffer.insert('\n');
            }
            Key::Backspace => {
                state.buffer.backspace();
            }
            Key::Left => {
                state.buffer.move_cursor_backward();
            }
            Key::Right => {
                state.buffer.move_cursor_forward();
            }
            Key::Up => {
                state.buffer.move_cursor_up();
            }
            Key::Down => {
                state.buffer.move_cursor_down();
            }
            Key::Char(ch) => {
                if !ch.is_control() {
                    state.buffer.insert(ch);
                }
            }
            _ => {}
        }
        state
    }
}

impl Mode for NormalMode {
    fn name(&self) -> &str {
        "normal"
    }

    fn handle(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Char('q') => {
                state.quit = true;
            }
            Key::Left => {
                state.buffer.move_cursor_backward();
            }
            Key::Right => {
                state.buffer.move_cursor_forward();
            }
            Key::Up => {
                state.buffer.move_cursor_up();
            }
            Key::Down => {
                state.buffer.move_cursor_down();
            }
            Key::Char('i') => {
                state.modes.push(Arc::new(InsertMode));
            }
            Key::Char('h') => {
                state.buffer.move_cursor(CoordUnaligned::backward);
            }
            Key::Char('H') => {
                state.buffer.extend_cursor(CoordUnaligned::backward);
            }
            Key::Char('l') => {
                state.buffer.move_cursor(CoordUnaligned::forward);
            }
            Key::Char('L') => {
                state.buffer.extend_cursor(CoordUnaligned::forward);
            }
            Key::Char('j') => {
                state.buffer.move_cursor(CoordUnaligned::down_unaligned);
            }
            Key::Char('J') => {
                state.buffer.extend_cursor(CoordUnaligned::down_unaligned);
            }
            Key::Char('k') => {
                state.buffer.move_cursor(CoordUnaligned::up_unaligned);
            }
            Key::Char('K') => {
                state.buffer.extend_cursor(CoordUnaligned::up_unaligned);
            }
            Key::Char('d') => {
                state.yanked = state.buffer.delete();
            }
            Key::Char('y') => {
                state.yanked = state.buffer.yank();
            }
            Key::Char('p') => {
                state.buffer.paste(&state.yanked);
            }
            Key::Char('w') => {
                state.buffer.move_cursor(CoordUnaligned::forward_word);
            }
            Key::Char('W') => {
                state.buffer.extend_cursor(CoordUnaligned::forward_word);
            }
            Key::Char('b') => {
                state.buffer.move_cursor(CoordUnaligned::backward_word);
            }
            Key::Char('B') => {
                state.buffer.extend_cursor(CoordUnaligned::backward_word);
            }
            Key::Char('x') => {
                state.buffer.move_line();
            }
            Key::Char('X') => {
                state.buffer.extend_line();
            }
            Key::Char('\'') | Key::Alt(';') => {
                state.buffer.reverse_selections();
            }
            _ => {}
        }
        state
    }
}
#[derive(Default, Clone)]
struct State {
    quit: bool,
    modes: Vec<Arc<Mode>>,
    buffer: Buffer,
    yanked: Vec<Rope>,
}

struct Breeze {
    state: State,
    screen: AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    display_cols: usize,
    display_rows: usize,
}

impl Breeze {
    fn init() -> Result<Self> {
        let screen = AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());

        let mut state = State::default();
        state.modes.push(Arc::new(NormalMode));
        let (cols, rows) = termion::terminal_size()?;
        Ok(Self {
            state,
            display_cols: cols as usize,
            display_rows: rows as usize,
            screen,
        })
    }

    fn run(&mut self) -> Result<()> {
        self.draw_buffer()?;
        self.screen.flush()?;

        let stdin = std::io::stdin();
        for c in stdin.keys() {
            match c {
                Ok(key) => {
                    self.state = self
                        .state
                        .modes
                        .last()
                        .expect("at least one mode")
                        .handle(self.state.clone(), key);
                }
                Err(e) => panic!("{}", e),
            }

            if self.state.quit {
                return Ok(());
            }
            self.draw_buffer()?;
            self.screen.flush()?;
        }
        Ok(())
    }

    fn draw_buffer(&mut self) -> Result<()> {
        write!(
            self.screen,
            "{}{}{}",
            color::Fg(color::Reset),
            color::Bg(color::Reset),
            termion::clear::All
        )?;
        let mut ch_idx = 0;
        for (line_i, line) in self
            .state
            .buffer
            .text
            .lines()
            .enumerate()
            .take(self.display_rows)
        {
            write!(
                self.screen,
                "{}",
                termion::cursor::Goto(1, line_i as u16 + 1)
            )?;
            for (char_i, ch) in line.chars().enumerate().take(self.display_cols) {
                let in_selection = self.state.buffer.is_idx_selected(Idx(ch_idx + char_i));
                let ch = if ch == '\n' {
                    if in_selection {
                        '·'
                    } else {
                        ' '
                    }
                } else {
                    ch
                };

                if in_selection {
                    write!(
                        self.screen,
                        "{}{}{}",
                        color::Fg(color::AnsiValue(16)),
                        color::Bg(color::AnsiValue(4)),
                        ch
                    )?;
                } else {
                    write!(
                        self.screen,
                        "{}{}{}",
                        color::Fg(color::Reset),
                        color::Bg(color::Reset),
                        ch
                    )?;
                }
            }
            ch_idx += line.len_chars();
        }

        // status
        write!(
            self.screen,
            "{}{}{}{}",
            color::Fg(color::Reset),
            color::Bg(color::Reset),
            termion::cursor::Goto(1, self.display_rows as u16),
            self.state.modes.last().unwrap().name(),
        )?;

        // cursor
        let cursor = self.state.buffer.cursor_pos();
        write!(
            self.screen,
            "\x1b[6 q{}{}",
            termion::cursor::Goto(cursor.column as u16 + 1, cursor.line as u16 + 1),
            termion::cursor::Show,
        )?;
        Ok(())
    }
}
fn main() -> Result<()> {
    Breeze::init()?.run()?;
    Ok(())
}
