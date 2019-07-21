use super::*;

#[derive(Clone, Debug, Default)]
pub struct Command {
    cmd: String,
}

impl Command {
    pub fn new() -> Self {
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

    fn render(&self, state: &State, mut render: &mut dyn Renderer) {
        let (_, status_rect) = super::default_render(self, state, render);
        let style = render.color_map().default;
        let mut status_view = status_rect.to_renderer(&mut render);
        status_view.print(
            render::Coord { x: 0, y: 0 },
            &format!(":{}", self.cmd),
            style,
        );
    }
}

impl Command {
    fn handle_command_complete(&self, state: &mut State) {
        let cmd: Vec<_> = self.cmd.split_whitespace().map(str::to_owned).collect();
        if cmd.is_empty() {
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
                    state.open_buffer(&PathBuf::from(s))
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
