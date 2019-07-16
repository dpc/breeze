use super::*;

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
