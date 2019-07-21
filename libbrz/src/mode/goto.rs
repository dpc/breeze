use super::*;

#[derive(Clone, Debug, Default)]
pub struct Goto;

impl Mode for Goto {
    fn name(&self) -> &str {
        "goto"
    }
    fn handle(&mut self, state: &mut State, key: Key) {
        state.set_mode(Normal::default());
        let buffer = state.cur_buffer_mut();
        match key {
            Key::Esc => {}
            Key::Char('l') => {
                buffer.move_cursor_coord(|coord, text| {
                    let line = text.line(coord.line);
                    coord.set_column(line.len_chars() - 1, text)
                });
            }
            Key::Char('h') => {
                buffer.move_cursor_coord(|coord, text| coord.set_column(0, text));
            }
            Key::Char('k') => {
                buffer.move_cursor_coord(|coord, text| {
                    coord.set_line(0, text).trim_column_to_buf(text)
                });
            }
            Key::Char('j') => {
                buffer.move_cursor_coord(|coord, text| {
                    coord
                        .set_line(text.len_lines().saturating_sub(1), text)
                        .trim_column_to_buf(text)
                });
            }
            Key::Char('i') => {
                buffer.move_cursor(|idx, text| idx.before_first_non_whitespace(text));
            }
            _ => {}
        }
    }
}
