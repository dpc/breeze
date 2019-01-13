mod prelude;

use self::prelude::*;

use std::sync::Arc;

use std::io::Write;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;

/// Selection
///
/// An ordererd pair of indices in the buffer
#[derive(Default, Debug, Clone)]
struct Selection {
    anchor: usize,
    cursor: usize,
}

impl Selection {
    fn is_idx_inside(&self, pos: usize) -> bool {
        let anchor = self.anchor;
        let cursor = self.cursor;

        if anchor < cursor {
            anchor <= pos && pos < cursor
        } else if cursor < anchor {
            cursor <= pos && pos < anchor
        } else {
            false
        }
    }

    fn is_forward(&self) -> Option<bool> {
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

    fn sorted(&self) -> (usize, usize) {
        if self.anchor < self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    fn sorted_range(&self) -> std::ops::Range<usize> {
        let (a, b) = self.sorted();
        a..b
    }

    fn collapse_to_cursor(&mut self) {
        self.anchor = self.cursor;
    }

    fn reverse(&mut self) {
        std::mem::swap(&mut self.cursor, &mut self.anchor);
    }
}

#[derive(Default, Debug, Clone)]
struct Buffer {
    text: ropey::Rope,
    sel: Selection,
}

impl Buffer {
    fn is_idx_selected(&self, idx: usize) -> bool {
        self.sel.is_idx_inside(idx)
    }

    fn reverse(&mut self) {
        self.sel.reverse();
    }

    fn insert(&mut self, ch: char) {
        self.text.insert_char(self.sel.cursor, ch);
        self.sel.anchor = self.sel.cursor;
        self.sel.cursor += 1;
    }

    fn delete(&mut self) {
        self.text.remove(self.sel.sorted_range());
        self.sel.collapse_to_cursor();
    }

    fn backspace(&mut self) {
        if self.sel.cursor == 0 {
            return;
        }

        self.text.remove(self.sel.cursor - 1..self.sel.cursor);
        match self.sel.is_forward() {
            Some(true) => {
                self.sel.cursor -= 1;
            }
            _ => {
                self.sel.anchor -= 1;
                self.sel.cursor -= 1;
            }
        }
    }

    fn move_left(&mut self) {
        if 0 < self.sel.cursor {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor -= 1;
        }
    }

    fn move_up(&mut self) {
        let (cur_line, cur_col) = self.idx_to_pos(self.sel.cursor);
        if 0 < cur_line {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor = self.pos_to_idx_trim(cur_line - 1, cur_col);
        }
    }

    fn move_down(&mut self) {
        let (cur_line, cur_col) = self.idx_to_pos(self.sel.cursor);
        if cur_line + 1 < self.text.len_lines() {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor = self.pos_to_idx_trim(cur_line + 1, cur_col);
        }
    }
    fn extend_left(&mut self) {
        if 0 < self.sel.cursor {
            self.sel.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.sel.cursor < self.text.len_chars() {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor += 1;
        }
    }

    fn extend_right(&mut self) {
        if self.sel.cursor < self.text.len_chars() {
            self.sel.cursor += 1;
        }
    }

    fn forward_word(&mut self) {
        let mut cursor = self.sel.cursor;
        for idx in cursor.. {
            if idx + 1 == self.text.len_chars() || self.text.char(idx).is_alphanumeric() {
                cursor = idx;
                break;
            }
        }

        for idx in cursor.. {
            if idx + 1 == self.text.len_chars() || !self.text.char(idx).is_alphanumeric() {
                cursor = idx;
                break;
            }
        }
        if cursor != self.sel.cursor {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor = cursor;
        }
    }

    fn cursor_pos(&self) -> (usize, usize) {
        self.idx_to_pos(self.sel.cursor)
    }

    fn idx_to_pos(&self, char_idx: usize) -> (usize, usize) {
        let line = self.text.char_to_line(char_idx);
        let line_start_pos = self.text.line_to_char(line);
        let col = char_idx - line_start_pos;

        (line, col)
    }

    fn pos_to_idx(&self, row: usize, col: usize) -> Option<usize> {
        let line = self.text.line(row);
        if line.len_chars() > col {
            None
        } else {
            Some(self.text.line_to_char(row) + col)
        }
    }

    fn pos_to_idx_trim(&self, row: usize, col: usize) -> usize {
        let line = self.text.line(row);
        let line_len = line.len_chars();
        if line_len == 0 {
            self.text.line_to_char(row)
        } else {
            self.text.line_to_char(row) + std::cmp::min(col, line_len - 1)
        }
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
                state.buffer.move_left();
            }
            Key::Right => {
                state.buffer.move_right();
            }
            Key::Char(ch) => {
                state.buffer.insert(ch);
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
            Key::Char('i') => {
                state.modes.push(Arc::new(InsertMode));
            }
            Key::Char('h') => {
                state.buffer.move_left();
            }
            Key::Char('H') => {
                state.buffer.extend_left();
            }
            Key::Char('l') => {
                state.buffer.move_right();
            }
            Key::Char('L') => {
                state.buffer.extend_right();
            }
            Key::Char('j') => {
                state.buffer.move_down();
            }
            Key::Char('k') => {
                state.buffer.move_up();
            }
            Key::Char('d') => {
                state.buffer.delete();
            }
            Key::Char('w') => {
                state.buffer.forward_word();
            }
            Key::Char('\'') | Key::Alt(';') => {
                state.buffer.reverse();
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
                let in_selection = self.state.buffer.is_idx_selected(ch_idx + char_i);
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
        let (cur_row, cur_col) = self.state.buffer.cursor_pos();
        write!(
            self.screen,
            "\x1b[6 q{}{}",
            termion::cursor::Goto(cur_col as u16 + 1, cur_row as u16 + 1),
            termion::cursor::Show,
        )?;
        Ok(())
    }
}
fn main() -> Result<()> {
    Breeze::init()?.run()?;
    Ok(())
}
