mod prelude;

use self::prelude::*;

use rustbox::{Color, Key, OutputMode, RustBox};
use std::sync::Arc;

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

    fn move_right(&mut self) {
        if self.sel.cursor < self.text.len_chars() {
            self.sel.anchor = self.sel.cursor;
            self.sel.cursor += 1;
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
}

trait Mode {
    fn handle(&self, state: State, key: rustbox::Key) -> State;
    fn name(&self) -> &str;
}

struct InsertMode;
struct NormalMode;

impl Mode for InsertMode {
    fn name(&self) -> &str {
        "insert"
    }

    fn handle(&self, mut state: State, key: rustbox::Key) -> State {
        match key {
            Key::Esc => {
                state.modes.pop();
            }
            Key::Enter => {
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

    fn handle(&self, mut state: State, key: rustbox::Key) -> State {
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
            Key::Char('l') => {
                state.buffer.move_right();
            }
            Key::Char('d') => {
                state.buffer.delete();
            }
            Key::Char('\'') => {
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
    rb: RustBox,
    display_cols: usize,
    display_rows: usize,
}

impl Breeze {
    fn init() -> Result<Self> {
        let mut rb = RustBox::init(Default::default())?;

        rb.set_output_mode(OutputMode::EightBit);
        let mut state = State::default();
        state.modes.push(Arc::new(NormalMode));
        Ok(Self {
            state,
            display_cols: rb.width(),
            display_rows: rb.height(),
            rb,
        })
    }

    fn run(&mut self) -> Result<()> {
        while !self.state.quit {
            self.draw_buffer()?;
            self.rb.present();
            self.run_one_event()?;
        }
        Ok(())
    }

    fn draw_buffer(&mut self) -> Result<()> {
        self.rb.clear();
        let mut ch_idx = 0;
        for (line_i, line) in self
            .state
            .buffer
            .text
            .lines()
            .enumerate()
            .take(self.display_rows)
        {
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

                self.rb.print_char(
                    char_i,
                    line_i,
                    rustbox::RB_NORMAL,
                    if in_selection {
                        Color::Byte(16)
                    } else {
                        Color::White
                    },
                    if in_selection {
                        Color::Byte(4)
                    } else {
                        Color::Black
                    },
                    ch,
                );
            }
            ch_idx += line.len_chars();
        }

        self.rb.print(
            0,
            self.display_rows - 1,
            rustbox::RB_NORMAL,
            Color::White,
            Color::Black,
            self.state.modes.last().unwrap().name(),
        );

        print!("\x1b[6 q");
        let (cur_row, cur_col) = self.state.buffer.cursor_pos();
        self.rb.set_cursor(cur_col as isize, cur_row as isize);
        Ok(())
    }

    fn run_one_event(&mut self) -> Result<()> {
        match self.rb.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                self.state = self
                    .state
                    .modes
                    .last()
                    .expect("at least one mode")
                    .handle(self.state.clone(), key);
            }
            Err(e) => panic!("{}", e),
            _ => {}
        }
        Ok(())
    }
}
fn main() -> Result<()> {
    Breeze::init()?.run()?;
    Ok(())
}
