use super::*;

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
