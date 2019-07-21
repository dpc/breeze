use super::*;

#[derive(Clone, Debug, Default)]
pub struct Insert;

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
                buffer.insert_enter();
            }
            Key::Char('\t') => {
                buffer.insert_tab();
            }
            Key::Backspace => {
                buffer.backspace();
            }
            Key::Left => {
                buffer.move_cursor_backward(1);
            }
            Key::Right => {
                buffer.move_cursor_forward(1);
            }
            Key::Up => {
                buffer.move_cursor_up(1);
            }
            Key::Down => {
                buffer.move_cursor_down(1);
            }
            Key::Char(ch) => {
                if !ch.is_control() {
                    buffer.insert_char(ch);
                }
            }
            _ => {}
        }
    }
}
