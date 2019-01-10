mod prelude;

use self::prelude::*;

use rustbox::{Color, Key, OutputMode, RustBox};

#[derive(Default, Debug, Clone)]
struct Buffer {
    text: ropey::Rope,
    anchor: usize,
    cursor: usize,
}

impl Buffer {
    fn is_pos_in_selection(&self, pos: usize) -> bool {
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

    fn is_selection_forward(&self) -> Option<bool> {
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

    fn insert(&mut self, ch: char) {
        self.text.insert_char(self.cursor, ch);
        self.anchor = self.cursor;
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.text.remove(self.cursor - 1..self.cursor);
        match self.is_selection_forward() {
            Some(true) => {
                self.cursor -= 1;
            }
            _ => {
                self.anchor -= 1;
                self.cursor -= 1;
            }
        }
    }
    fn move_left(&mut self) {
        if 0 < self.cursor {
            self.anchor = self.cursor;
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.text.len_chars() {
            self.anchor = self.cursor;
            self.cursor += 1;
        }
    }

    fn char_idx_to_row_col(&self, char_idx: usize) -> (usize, usize) {
        let line = self.text.char_to_line(char_idx);
        let line_start_pos = self.text.line_to_char(line);
        let col = char_idx - line_start_pos;

        (line, col)
    }

    fn cursor_pos(&self) -> (usize, usize) {
        self.char_idx_to_row_col(self.cursor)
    }
}

struct Breeze {
    running: bool,
    rb: RustBox,
    buffer: Buffer,
    display_cols: usize,
    display_rows: usize,
}

impl Breeze {
    fn init() -> Result<Self> {
        let mut rb = RustBox::init(Default::default())?;

        rb.set_output_mode(OutputMode::EightBit);
        Ok(Self {
            display_cols: rb.width(),
            display_rows: rb.height(),
            running: true,
            rb,
            buffer: default(),
        })
    }

    fn run(&mut self) -> Result<()> {
        while self.running {
            self.draw_buffer()?;
            self.rb.present();
            self.run_one_event()?;
        }
        Ok(())
    }

    fn clear_buffer(&self) {
        // for whatever reason `RustBox::clear` is not functional
        for x in 0..self.display_cols {
            for y in 0..self.display_rows {
                unsafe {
                    self.rb.change_cell(
                        x,
                        y,
                        '-' as u32,
                        Color::White.as_256color(),
                        Color::Black.as_256color(),
                    );
                }
            }
        }
    }
    fn draw_buffer(&mut self) -> Result<()> {
        // self.clear_buffer();
        self.rb.clear();
        let mut ch_idx = 0;
        for (line_i, line) in self.buffer.text.lines().enumerate().take(self.display_rows) {
            for (char_i, ch) in line.chars().enumerate().take(self.display_cols) {
                let in_selection = self.buffer.is_pos_in_selection(ch_idx + char_i);
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

        print!("\x1b[6 q");
        let (cur_row, cur_col) = self.buffer.cursor_pos();
        self.rb.set_cursor(cur_col as isize, cur_row as isize);
        Ok(())
    }

    fn run_one_event(&mut self) -> Result<()> {
        match self.rb.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => match key {
                Key::Char('q') => {
                    self.running = false;
                }
                Key::Char(ch) if 'a' <= ch && ch <= 'z' => {
                    self.buffer.insert(ch);
                }
                Key::Char(ch) if 'A' <= ch && ch <= 'Z' => {
                    self.buffer.insert(ch);
                }
                Key::Char(ch) if '1' <= ch && ch <= '9' => {
                    self.buffer.insert(ch);
                }
                Key::Enter => {
                    self.buffer.insert('\n');
                }
                Key::Backspace => {
                    self.buffer.backspace();
                }
                Key::Left => {
                    self.buffer.move_left();
                }
                Key::Right => {
                    self.buffer.move_right();
                }
                _ => {}
            },
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
