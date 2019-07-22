use super::*;

use crate::action;
use crate::state::State;
use crate::NaturalyOrderedKey;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Normal {
    num_prefix: Option<usize>,
}

impl Mode for Normal {
    fn name(&self) -> &str {
        "normal"
    }

    fn handle(&mut self, state: &mut State, key: Key) {
        if state.cur_buffer_opt().is_none() {
            match key {
                Key::Char(':') => {
                    state.set_mode(Command::new());
                }
                Key::Ctrl('p') => {
                    state.set_mode(Find::default());
                }
                _ => {}
            }
            return;
        }

        match key {
            Key::Char(n @ '0'..='9') => {
                self.num_prefix = Some(
                    self.num_prefix
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(n as usize - '0' as usize),
                );
            }
            other => {
                state
                    .cur_buffer_state_mut_opt()
                    .map(|b| b.maybe_commit_undo_point());
                self.handle_not_digit(state, other);
                self.num_prefix = None;
                state
                    .cur_buffer_state_mut_opt()
                    .map(|b| b.maybe_commit_undo_point());
            }
        }
    }

    fn available_actions(&self) -> &action::Map {
        action::normal::all_actions()
    }
}

impl Normal {
    fn handle_not_digit(&self, state: &mut State, key: Key) -> bool {
        let times = self.num_prefix.unwrap_or(1);

        match key {
            Key::Char('u') => {
                state.cur_buffer_state_mut().undo(times);
                true
            }
            Key::Char('U') => {
                state.cur_buffer_state_mut().redo(times);
                true
            }
            other => self.handle_not_digit_not_undo(state, other),
        }
    }

    fn handle_not_digit_not_undo(&self, state: &mut State, key: Key) -> bool {
        let times = self.num_prefix.unwrap_or(1);
        let buffer = state.cur_buffer_mut();
        match key {
            Key::Esc => {}
            Key::Char(' ') => {
                buffer.collapse();
            }
            Key::Ctrl('p') => {
                state.set_mode(Find::default());
            }
            Key::Char('g') => {
                if let Some(num_prefix) = self.num_prefix {
                    state.cur_buffer_mut().move_cursor_coord(|coord, text| {
                        coord.set_line(num_prefix.saturating_sub(1), text)
                    });
                } else {
                    state.set_mode(Goto)
                }
            }
            Key::Left => {
                state.cur_buffer_mut().move_cursor_backward(times);
            }
            Key::Right => {
                state.cur_buffer_mut().move_cursor_forward(times);
            }
            Key::Up => {
                state.cur_buffer_mut().move_cursor_up(times);
            }
            Key::Down => {
                state.cur_buffer_mut().move_cursor_down(times);
            }
            Key::Char('h') => {
                state.cur_buffer_mut().move_cursor_backward(times);
            }
            Key::Char('H') => {
                state.cur_buffer_mut().extend_cursor_backward(times);
            }
            Key::Char('l') => {
                state.cur_buffer_mut().move_cursor_forward(times);
            }
            Key::Char('L') => {
                state.cur_buffer_mut().extend_cursor_forward(times);
            }
            Key::Char('j') => {
                state.cur_buffer_mut().move_cursor_down(times);
            }
            Key::Char('J') => {
                state.cur_buffer_mut().extend_cursor_down(times);
            }
            Key::Char('k') => {
                state.cur_buffer_mut().move_cursor_up(times);
            }
            Key::Char('K') => {
                state.cur_buffer_mut().extend_cursor_up(times);
            }
            Key::Char('d') => {
                state.yanked = state.cur_buffer_mut().delete();
            }
            Key::Char('c') => {
                state.yanked = state.cur_buffer_mut().delete();
                state.set_mode(Insert);
            }
            Key::Char('o') => {
                state.cur_buffer_mut().open();
                state.set_mode(Insert);
            }
            Key::Char('y') => {
                state.yanked = state.cur_buffer_mut().yank();
            }
            Key::Char('p') => {
                let yanked = state.yanked.clone();
                state.cur_buffer_mut().paste(&yanked);
            }
            Key::Char('P') => {
                let yanked = state.yanked.clone();
                state.cur_buffer_mut().paste_extend(&yanked);
            }
            Key::Char('w') => {
                state.cur_buffer_mut().move_cursor_2(Idx::forward_word);
            }
            Key::Char('W') => {
                state.cur_buffer_mut().extend_cursor_2(Idx::forward_word);
            }
            Key::Char('b') => {
                state.cur_buffer_mut().move_cursor_2(Idx::backward_word);
            }
            Key::Char('B') => {
                state.cur_buffer_mut().extend_cursor_2(Idx::backward_word);
            }
            Key::Char('x') => {
                state.cur_buffer_mut().move_line();
            }
            Key::Char('X') => {
                state.cur_buffer_mut().extend_line();
            }
            Key::Char('%') => {
                state.cur_buffer_mut().select_all();
            }
            Key::Char('\'') | Key::Alt(';') => {
                state.cur_buffer_mut().reverse_selections();
            }
            Key::Char('>') => {
                state.cur_buffer_mut().increase_indent(times);
            }
            Key::Char('<') => {
                state.cur_buffer_mut().decrease_indent(times);
            }
            key => {
                if let Some(action) = action::normal::all_actions().get(&NaturalyOrderedKey(key)) {
                    action.execute(state);
                } else {
                    return false;
                }
            }
        }
        true
    }
}
