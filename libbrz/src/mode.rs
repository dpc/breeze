use crate::idx::*;
use crate::state::State;
use crate::Key;
use default::default;
use std::cmp;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Normal {
    num_prefix: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal(Normal),
    Insert,
    Goto,
    Command,
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
            Command => "command",
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
            Command => self.handle_command_key(state, key),
            Goto => self.handle_goto(state, key),
        }
    }

    fn handle_command_key(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Esc => {
                state.cmd = "".into();
                state.set_mode(default());
            }
            Key::Char('\n') => {
                self.handle_command(&mut state);
                state.cmd = "".into();
                state.set_mode(default());
            }
            Key::Char(ch) => {
                state.cmd.push(ch);
            }
            _ => {}
        }
        state
    }

    fn handle_command(&self, state: &mut State) {
        let cmd: Vec<_> = state.cmd.split_whitespace().map(str::to_owned).collect();
        if cmd.len() < 1 {
            return;
        }

        match cmd[0].as_str() {
            "q" => {
                state.quit = true;
            }
            "bn" => {
                state.buffer_next();
            }
            "bp" => {
                state.buffer_prev();
            }
            "e" => {
                for s in &cmd[1..] {
                    state.open_buffer(PathBuf::from(s))
                }
            }
            "db" => {
                state.delete_buffer();
            }
            "w" => {
                state.write_buffer(cmd.get(1).map(PathBuf::from));
            }
            _ => state.msg = Some(format!("unrecognized command: {}", state.cmd)),
        }
    }

    fn handle_insert(&self, mut state: State, key: Key) -> State {
        match key {
            Key::Esc => {
                state.set_mode(default());
            }
            Key::Char('\n') => {
                state.cur_buffer_mut().insert_enter();
            }
            Key::Char('\t') => {
                state.cur_buffer_mut().insert_tab();
            }
            Key::Backspace => {
                state.cur_buffer_mut().backspace();
            }
            Key::Left => {
                state.cur_buffer_mut().move_cursor_backward(1);
            }
            Key::Right => {
                state.cur_buffer_mut().move_cursor_forward(1);
            }
            Key::Up => {
                state.cur_buffer_mut().move_cursor_up(1);
            }
            Key::Down => {
                state.cur_buffer_mut().move_cursor_down(1);
            }
            Key::Char(ch) => {
                if !ch.is_control() {
                    state.cur_buffer_mut().insert_char(ch);
                }
            }
            _ => {}
        }
        state
    }

    fn handle_goto(&self, mut state: State, key: Key) -> State {
        state.set_mode(default());
        match key {
            Key::Esc => {}
            Key::Char('l') => {
                state.cur_buffer_mut().move_cursor_coord(|coord, text| {
                    let line = text.line(coord.line);
                    coord.set_column(line.len_chars() - 1, text)
                });
            }
            Key::Char('h') => {
                state
                    .cur_buffer_mut()
                    .move_cursor_coord(|coord, text| coord.set_column(0, text));
            }
            Key::Char('k') => {
                state.cur_buffer_mut().move_cursor_coord(|coord, text| {
                    coord.set_line(0, text).trim_column_to_buf(text).into()
                });
            }
            Key::Char('j') => {
                state.cur_buffer_mut().move_cursor_coord(|coord, text| {
                    coord
                        .set_line(text.len_lines().saturating_sub(1), text)
                        .trim_column_to_buf(text)
                        .into()
                });
            }
            Key::Char('i') => {
                state
                    .cur_buffer_mut()
                    .move_cursor(|idx, text| idx.before_first_non_whitespace(text));
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
                state.set_mode(Mode::Normal(Normal {
                    num_prefix: Some(
                        self.num_prefix
                            .unwrap_or(0)
                            .saturating_mul(10)
                            .saturating_add(n as usize - '0' as usize),
                    ),
                }));
                state
            }
            other => {
                let mut state = self.handle_not_digit(state, other);
                if let &mut Mode::Normal(ref mut normal) = state.get_mode() {
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
                if let Some(buffer_history_undo_i) =
                    state.cur_buffer_state_mut().buffer_history_undo_i.as_mut()
                {
                    let history_i = buffer_history_undo_i.saturating_sub(1);
                    *buffer_history_undo_i = history_i;
                    state.cur_buffer_state_mut().buffer =
                        state.cur_buffer_state().buffer_history[history_i].clone();
                } else if !state.cur_buffer_state().buffer_history.is_empty() {
                    let history_i = state
                        .cur_buffer_state()
                        .buffer_history
                        .len()
                        .saturating_sub(1);
                    state.cur_buffer_state_mut().buffer_history_undo_i = Some(history_i);
                    state.cur_buffer_state_mut().buffer =
                        state.cur_buffer_state().buffer_history[history_i].clone();
                }
                state
            }
            Key::Char('U') => {
                let buffer_state = state.cur_buffer_state_mut();
                if let Some(buffer_history_undo_i) = buffer_state.buffer_history_undo_i.as_mut() {
                    let history_i = cmp::min(
                        buffer_history_undo_i.saturating_add(times),
                        buffer_state.buffer_history.len() - 1,
                    );
                    *buffer_history_undo_i = history_i;
                    buffer_state.buffer = buffer_state.buffer_history[history_i].clone();
                }
                state
            }
            other => {
                let prev_buf = state.cur_buffer().clone();
                let mut state = self.handle_not_digit_not_undo(state.clone(), other);

                state
                    .cur_buffer_state_mut()
                    .maybe_commit_undo_point(&prev_buf);
                state
            }
        }
    }

    fn handle_not_digit_not_undo(&self, mut state: State, key: Key) -> State {
        let times = self.num_prefix.unwrap_or(1);
        match key {
            Key::Esc => {
                state.set_mode(Mode::default());
            }
            Key::Char(' ') => {
                state.cur_buffer_mut().collapse();
            }
            Key::Char(':') => {
                state.set_mode(Mode::Command);
            }
            Key::Char('g') => {
                if let Some(num_prefix) = self.num_prefix {
                    state.cur_buffer_mut().move_cursor_coord(|coord, text| {
                        coord.set_line(num_prefix.saturating_sub(1), text)
                    });
                    state.set_mode(Mode::default());
                } else {
                    state.set_mode(Mode::Goto)
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
            Key::Char('i') => {
                state.set_mode(Mode::Insert);
                state.cur_buffer_state_mut().commit_undo_point();
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
                state.set_mode(self::Mode::Insert);
            }
            Key::Char('o') => {
                state.cur_buffer_mut().open();
                state.set_mode(self::Mode::Insert);
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
            Key::Ctrl('d') => {
                state.cur_buffer_mut().move_cursor_down(25);
            }
            Key::Ctrl('D') => {
                state.cur_buffer_mut().extend_cursor_down(25);
            }
            Key::Ctrl('u') => {
                state.cur_buffer_mut().move_cursor_up(25);
            }
            Key::Ctrl('U') => {
                state.cur_buffer_mut().extend_cursor_up(25);
            }
            Key::Char('>') => {
                state.cur_buffer_mut().increase_indent(times);
            }
            Key::Char('<') => {
                state.cur_buffer_mut().decrease_indent(times);
            }
            _ => {}
        }
        state
    }
}
