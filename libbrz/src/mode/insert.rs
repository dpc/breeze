use super::*;

#[derive(Clone, Debug, Default)]
pub struct Insert {
    extend: bool,
}

impl Insert {
    pub fn new_normal() -> Self {
        Self { extend: false }
    }

    pub fn new_extend() -> Self {
        Self { extend: true }
    }
}

impl Mode for Insert {
    fn name(&self) -> &str {
        "insert"
    }
    fn handle(&mut self, state: &mut State, key: Key) {
        let buffer = state.cur_buffer_mut();
        match key {
            Key::Esc => {
                state.set_mode(Normal::default());
            }
            Key::Char('\n') => {
                buffer.insert_enter(self.extend);
            }
            Key::Char('\t') => {
                buffer.insert_tab(self.extend);
            }
            Key::Backspace => {
                buffer.backspace(self.extend);
            }
            Key::Left => {
                if self.extend {
                    buffer.extend_cursor_backward(1);
                } else {
                    buffer.move_cursor_backward(1);
                }
            }
            Key::Right => {
                if self.extend {
                    buffer.extend_cursor_forward(1);
                } else {
                    buffer.move_cursor_forward(1);
                }
            }
            Key::Up => {
                if self.extend {
                    buffer.extend_cursor_up(1);
                } else {
                    buffer.move_cursor_up(1);
                }
            }
            Key::Down => {
                if self.extend {
                    buffer.extend_cursor_down(1);
                } else {
                    buffer.move_cursor_down(1);
                }
            }
            Key::Char(ch) => {
                if !ch.is_control() {
                    buffer.insert_char(ch, self.extend);
                }
            }
            _ => {}
        }
    }
}
