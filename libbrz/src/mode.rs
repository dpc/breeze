use crate::action;
use crate::idx::*;
use crate::state::State;
use crate::Key;
use std::path::PathBuf;

// TODO: mode should render itself, ha!
pub trait Mode {
    fn name(&self) -> &str;
    fn cmd_string(&self) -> Option<String> {
        None
    }
    fn handle(&mut self, state: &mut State, key: Key);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Normal {
    num_prefix: Option<usize>,
}

impl Mode for Normal {
    fn name(&self) -> &str {
        "normal"
    }

    fn handle(&mut self, state: &mut State, key: Key) {
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
                state.cur_buffer_state_mut().maybe_commit_undo_point();
                self.handle_not_digit(state, other);
                self.num_prefix = None;
                state.cur_buffer_state_mut().maybe_commit_undo_point();
            }
        }
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
        match key {
            Key::Esc => {}
            Key::Char(' ') => {
                state.cur_buffer_mut().collapse();
            }
            Key::Char(':') => {
                state.set_mode(Command::new());
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
            Key::Ctrl('P') => {
                state.set_mode(Find::default());
            }
            Key::Char('A') => {
                state
                    .cur_buffer_mut()
                    .extend_cursor(Idx::forward_to_line_end);
                state.set_mode(Insert);
            }
            key => {
                if let Some(action) = action::normal::all_actions().get(&key) {
                    action.execute(state);
                } else {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Debug, Default)]
pub struct Command {
    cmd: String,
}

impl Command {
    fn new() -> Self {
        Self { cmd: String::new() }
    }
}

impl Mode for Command {
    fn name(&self) -> &str {
        "command"
    }

    fn cmd_string(&self) -> Option<String> {
        Some(format!(":{}", self.cmd))
    }

    fn handle(&mut self, state: &mut State, key: Key) {
        match key {
            Key::Esc => {
                state.set_mode(Normal::default());
            }
            Key::Char('\n') => {
                self.handle_command_complete(state);
                state.set_mode(Normal::default());
            }
            Key::Char(ch) => {
                self.cmd.push(ch);
            }
            Key::Backspace => {
                self.cmd.pop();
            }
            _ => {}
        }
    }
}

impl Command {
    fn handle_command_complete(&self, state: &mut State) {
        let cmd: Vec<_> = self.cmd.split_whitespace().map(str::to_owned).collect();
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
            _ => state.msg = Some(format!("unrecognized command: {}", self.cmd)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Insert;

impl Mode for Insert {
    fn name(&self) -> &str {
        "insert"
    }
    fn handle(&mut self, state: &mut State, key: Key) {
        match key {
            Key::Esc => {
                state.set_mode(Normal::default());
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
    }
}

#[derive(Clone, Debug, Default)]
pub struct Goto;

impl Mode for Goto {
    fn name(&self) -> &str {
        "goto"
    }
    fn handle(&mut self, state: &mut State, key: Key) {
        state.set_mode(Normal::default());
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
    }
}

#[derive(Default, Debug, Clone)]
pub struct Find {
    s: String,
}

impl Mode for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn cmd_string(&self) -> Option<String> {
        Some(format!("/{}", self.s))
    }
    fn handle(&mut self, state: &mut State, key: Key) {
        match key {
            Key::Esc => {
                self.s = "".into();
                state.set_mode(Normal::default());
            }
            Key::Backspace => {
                self.s.pop();
            }
            Key::Char('\n') => {
                if let Some(path) = state.find_result.take() {
                    state.open_buffer(path);
                }
                self.s = "".into();
                state.set_mode(Normal::default());
            }
            Key::Char(ch) => {
                self.s.push(ch);
                state.find_result = (state.find_handler)(&self.s)
                    .ok()
                    .and_then(|v| v.first().cloned());
            }
            _ => {}
        }
    }
}
