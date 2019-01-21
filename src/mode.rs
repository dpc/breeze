use crate::coord::*;
use crate::State;
use default::default;
use termion::event::Key;

#[derive(Clone, Debug, Default)]
pub struct Normal {
    num_prefix: usize,
}

#[derive(Clone, Debug)]
pub enum Mode {
    Normal(Normal),
    Insert,
    Goto,
}

impl Default for Mode {
    fn default() -> Self {
        self::Mode::Normal(default())
    }
}

impl Mode {
    pub fn name(&self) -> &str {
        use self::Mode::*;
        match self {
            Normal(_) => "normal",
            Insert => "insert",
            Goto => "goto",
        }
    }

    pub fn get_num_prefix(&self) -> Option<usize> {
        if let Mode::Normal(normal) = self {
            Some(normal.num_prefix)
        } else {
            None
        }
    }

    pub fn num_prefix_str(&self) -> String {
        self.get_num_prefix()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "".into())
    }

    pub fn handle(&self, state: State, key: Key) -> State {
        use self::Mode::*;
        match self {
            Normal(normal) => normal.handle(state, key),
            Insert => self.handle_insert(state, key),
            Goto => self.handle_goto(state, key),
        }
    }

    fn handle_insert(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Esc => {
                state.mode = default();
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

    fn handle_goto(&self, mut state: State, key: Key) -> State {
        state.mode = default();
        match key {
            Key::Esc => {}
            Key::Char('l') => {
                state.buffer.move_cursor(|coord, text| {
                    let line = text.line(coord.line);
                    coord.set_column(line.len_chars() - 1, text)
                });
            }
            Key::Char('h') => {
                state
                    .buffer
                    .move_cursor(|coord, text| coord.set_column(0, text));
            }
            Key::Char('k') => {
                state.buffer.move_cursor(|coord, text| {
                    coord.set_line(0, text).trim_column_to_buf(text).into()
                });
            }
            Key::Char('j') => {
                state.buffer.move_cursor(|coord, text| {
                    coord
                        .set_line(text.len_lines().saturating_sub(1), text)
                        .trim_column_to_buf(text)
                        .into()
                });
            }
            Key::Char('i') => {
                state.buffer.move_cursor(|coord, text| {
                    let line = text.line(coord.line);
                    if let Some((i, _)) = line
                        .chars()
                        .enumerate()
                        .find(|(_i, ch)| !ch.is_whitespace())
                    {
                        coord.set_column(i, text)
                    } else {
                        coord.set_column(line.len_chars() - 1, text)
                    }
                });
            }
            _ => {}
        }
        state
    }
}

impl Normal {
    fn handle(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Esc => {
                state.mode = Mode::default();
            }
            Key::Char('q') => {
                state.quit = true;
            }
            Key::Char('g') => {
                if self.num_prefix > 0 {
                    state
                        .buffer
                        .move_cursor(|coord, text| coord.set_line(self.num_prefix, text));
                    state.mode = Mode::default();
                } else {
                    state.mode = Mode::Goto
                }
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
                state.mode = crate::Mode::Insert;
            }
            Key::Char('h') => {
                state.buffer.move_cursor(Coord::backward);
            }
            Key::Char('H') => {
                state.buffer.extend_cursor(Coord::backward);
            }
            Key::Char('l') => {
                state.buffer.move_cursor(Coord::forward);
            }
            Key::Char('L') => {
                state.buffer.extend_cursor(Coord::forward);
            }
            Key::Char('j') => {
                state.buffer.move_cursor(Coord::down_unaligned);
            }
            Key::Char('J') => {
                state.buffer.extend_cursor(Coord::down_unaligned);
            }
            Key::Char('k') => {
                state.buffer.move_cursor(Coord::up_unaligned);
            }
            Key::Char('K') => {
                state.buffer.extend_cursor(Coord::up_unaligned);
            }
            Key::Char('d') => {
                state.yanked = state.buffer.delete();
            }
            Key::Char('c') => {
                state.yanked = state.buffer.delete();
                state.mode = self::Mode::Insert;
            }
            Key::Char('y') => {
                state.yanked = state.buffer.yank();
            }
            Key::Char('p') => {
                state.buffer.paste(&state.yanked);
            }
            Key::Char('P') => {
                state.buffer.paste_extend(&state.yanked);
            }
            Key::Char('w') => {
                state.buffer.move_cursor_2(Coord::forward_word);
            }
            Key::Char('W') => {
                state.buffer.extend_cursor_2(Coord::forward_word);
            }
            Key::Char('b') => {
                state.buffer.move_cursor_2(Coord::backward_word);
            }
            Key::Char('B') => {
                state.buffer.extend_cursor_2(Coord::backward_word);
            }
            Key::Char('x') => {
                state.buffer.move_line();
            }
            Key::Char('X') => {
                state.buffer.extend_line();
            }
            Key::Char('%') => {
                state.buffer.select_all();
            }
            Key::Char('\'') | Key::Alt(';') => {
                state.buffer.reverse_selections();
            }
            Key::Char(n @ '0'..='9') => {
                state.mode = Mode::Normal(Normal {
                    num_prefix: self
                        .num_prefix
                        .saturating_mul(10)
                        .saturating_add(n as usize - '0' as usize),
                })
            }
            _ => {}
        }
        state
    }
}
