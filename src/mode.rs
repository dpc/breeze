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
        }
    }
    pub fn handle(&self, state: State, key: Key) -> State {
        use self::Mode::*;
        match self {
            Normal(normal) => normal.handle(state, key),
            Insert => self.handle_insert(state, key),
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
}

impl Normal {
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
                state.mode = crate::Mode::Insert;
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
                state.buffer.move_cursor_2(CoordUnaligned::forward_word);
            }
            Key::Char('W') => {
                state.buffer.extend_cursor_2(CoordUnaligned::forward_word);
            }
            Key::Char('b') => {
                state.buffer.move_cursor_2(CoordUnaligned::backward_word);
            }
            Key::Char('B') => {
                state.buffer.extend_cursor_2(CoordUnaligned::backward_word);
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
                    num_prefix: self.num_prefix * 10 + n as usize - '0' as usize,
                })
            }
            _ => {}
        }
        state
    }
}
