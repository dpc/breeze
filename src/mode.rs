use crate::coord::*;
use crate::State;
use std::sync::Arc;
use termion::event::Key;

/// Mode handles keypresses
pub trait Mode {
    /// Transform state into next state
    fn handle(&self, state: State, key: Key) -> State;
    fn name(&self) -> &str;
}

struct InsertMode;
pub struct NormalMode;

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
            Key::Char('c') => {
                state.yanked = state.buffer.delete();
                state.modes.push(Arc::new(InsertMode));
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
            Key::Char('\'') | Key::Alt(';') => {
                state.buffer.reverse_selections();
            }
            _ => {}
        }
        state
    }
}
