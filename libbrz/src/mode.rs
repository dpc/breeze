use crate::idx::*;
use crate::Key;
use crate::State;
use default::default;
use std::cmp;

#[derive(Clone, Debug, Default)]
pub struct Normal {
    num_prefix: Option<usize>,
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
            normal.num_prefix
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
                state.buffer.open();
            }
            Key::Char('\t') => {
                state.buffer.insert_tab();
            }
            Key::Backspace => {
                state.buffer.backspace();
            }
            Key::Left => {
                state.buffer.move_cursor_backward(1);
            }
            Key::Right => {
                state.buffer.move_cursor_forward(1);
            }
            Key::Up => {
                state.buffer.move_cursor_up(1);
            }
            Key::Down => {
                state.buffer.move_cursor_down(1);
            }
            Key::Char(ch) => {
                if !ch.is_control() {
                    state.buffer.insert_char(ch);
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
                state.buffer.move_cursor_coord(|coord, text| {
                    let line = text.line(coord.line);
                    coord.set_column(line.len_chars() - 1, text)
                });
            }
            Key::Char('h') => {
                state
                    .buffer
                    .move_cursor_coord(|coord, text| coord.set_column(0, text));
            }
            Key::Char('k') => {
                state.buffer.move_cursor_coord(|coord, text| {
                    coord.set_line(0, text).trim_column_to_buf(text).into()
                });
            }
            Key::Char('j') => {
                state.buffer.move_cursor_coord(|coord, text| {
                    coord
                        .set_line(text.len_lines().saturating_sub(1), text)
                        .trim_column_to_buf(text)
                        .into()
                });
            }
            Key::Char('i') => {
                state
                    .buffer
                    .move_cursor_coord(|coord, text| coord.after_leading_whitespace(text));
            }
            _ => {}
        }
        state
    }
}

impl Normal {
    fn handle(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Char(n @ '0'..='9') => {
                state.mode = Mode::Normal(Normal {
                    num_prefix: Some(
                        self.num_prefix
                            .unwrap_or(0)
                            .saturating_mul(10)
                            .saturating_add(n as usize - '0' as usize),
                    ),
                });
                state
            }
            other => {
                let mut state = self.handle_not_digit(state, other);
                if let Mode::Normal(ref mut normal) = state.mode {
                    normal.num_prefix = None;
                }
                state
            }
        }
    }

    fn handle_not_digit(&self, mut state: State, key: Key) -> State {
        let times = self.num_prefix.unwrap_or(1);

        match key {
            Key::Char('u') => {
                if let Some(buffer_history_undo_i) = state.buffer_history_undo_i.as_mut() {
                    let history_i = buffer_history_undo_i.saturating_sub(1);
                    *buffer_history_undo_i = history_i;
                    state.buffer = state.buffer_history[history_i].clone();
                } else if !state.buffer_history.is_empty() {
                    let history_i = state.buffer_history.len().saturating_sub(1);
                    state.buffer_history_undo_i = Some(history_i);
                    state.buffer = state.buffer_history[history_i].clone();
                }
                state
            }
            Key::Char('U') => {
                if let Some(buffer_history_undo_i) = state.buffer_history_undo_i.as_mut() {
                    let history_i = cmp::min(
                        buffer_history_undo_i.saturating_add(times),
                        state.buffer_history.len() - 1,
                    );
                    *buffer_history_undo_i = history_i;
                    state.buffer = state.buffer_history[history_i].clone();
                }
                state
            }
            other => {
                let prev_buf = state.buffer.clone();
                self.handle_not_digit_not_undo(state, other)
                    .maybe_commit_undo_point(&prev_buf)
            }
        }
    }

    fn handle_not_digit_not_undo(&self, mut state: State, key: Key) -> State {
        let times = self.num_prefix.unwrap_or(1);
        match key {
            Key::Esc => {
                state.mode = Mode::default();
            }
            Key::Char(' ') => {
                state.buffer.collapse();
            }
            Key::Char('q') => {
                state.quit = true;
            }
            Key::Char('g') => {
                if let Some(num_prefix) = self.num_prefix {
                    state.buffer.move_cursor_coord(|coord, text| {
                        coord.set_line(num_prefix.saturating_sub(1), text)
                    });
                    state.mode = Mode::default();
                } else {
                    state.mode = Mode::Goto
                }
            }
            Key::Left => {
                state.buffer.move_cursor_backward(times);
            }
            Key::Right => {
                state.buffer.move_cursor_forward(times);
            }
            Key::Up => {
                state.buffer.move_cursor_up(times);
            }
            Key::Down => {
                state.buffer.move_cursor_down(times);
            }
            Key::Char('i') => {
                state.mode = Mode::Insert;
                state = state.commit_undo_point()
            }
            Key::Char('h') => {
                state.buffer.move_cursor_backward(times);
            }
            Key::Char('H') => {
                state.buffer.extend_cursor_backward(times);
            }
            Key::Char('l') => {
                state.buffer.move_cursor_forward(times);
            }
            Key::Char('L') => {
                state.buffer.extend_cursor_forward(times);
            }
            Key::Char('j') => {
                state.buffer.move_cursor_down(times);
            }
            Key::Char('J') => {
                state.buffer.extend_cursor_down(times);
            }
            Key::Char('k') => {
                state.buffer.move_cursor_up(times);
            }
            Key::Char('K') => {
                state.buffer.extend_cursor_up(times);
            }
            Key::Char('d') => {
                state.yanked = state.buffer.delete();
            }
            Key::Char('c') => {
                state.yanked = state.buffer.delete();
                state.mode = self::Mode::Insert;
            }
            Key::Char('o') => {
                state.buffer.open();
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
                state.buffer.move_cursor_2(Idx::forward_word);
            }
            Key::Char('W') => {
                state.buffer.extend_cursor_2(Idx::forward_word);
            }
            Key::Char('b') => {
                state.buffer.move_cursor_2(Idx::backward_word);
            }
            Key::Char('B') => {
                state.buffer.extend_cursor_2(Idx::backward_word);
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
            Key::Ctrl('d') => {
                state.buffer.move_cursor_down(25);
            }
            Key::Ctrl('D') => {
                state.buffer.extend_cursor_down(25);
            }
            Key::Ctrl('u') => {
                state.buffer.move_cursor_up(25);
            }
            Key::Ctrl('U') => {
                state.buffer.extend_cursor_up(25);
            }
            Key::Char('>') => {
                state.buffer.increase_indent(times);
            }
            Key::Char('<') => {
                state.buffer.decrease_indent(times);
            }
            _ => {}
        }
        state
    }
}
