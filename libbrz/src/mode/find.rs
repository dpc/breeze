use super::*;

#[derive(Default, Debug, Clone)]
pub struct Find {
    match_str: String,
    cur_matches: Vec<PathBuf>,
}

impl Find {
    fn update_matches(&mut self, state: &State) {
        self.cur_matches = (state.find_handler)(&self.match_str)
            .ok()
            .unwrap_or_else(|| vec![]);
    }
}
impl Mode for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn on_enter(&mut self, state: &State) {
        self.update_matches(state);
    }

    fn handle(&mut self, state: &mut State, key: Key) {
        match key {
            Key::Esc => {
                self.match_str = "".into();
                state.set_mode(Normal::default());
                return;
            }
            Key::Backspace => {
                self.match_str.pop();
            }
            Key::Char('\n') => {
                if let Some(path) = self.cur_matches.get(0) {
                    state.open_buffer(path);
                }
                self.match_str = "".into();
                state.set_mode(Normal::default());
            }
            Key::Char(ch) => {
                self.match_str.push(ch);
            }
            _ => return,
        }
        self.update_matches(state)
    }

    fn render(&self, state: &State, mut render: &mut dyn Renderer) {
        let dimensions = render.dimensions();
        let (buffer_rect, status_rect) = super::default_render_split_status_rect(render);
        let (buffer_rect, results_rect) =
            buffer_rect.split_horizontaly_at(-(dimensions.y as isize / 4));
        state.render_buffer(&mut buffer_rect.to_renderer(&mut render));

        default_render_status(self, render, status_rect);

        let style_default = render.color_map().default;
        let style_selected = render.color_map().selection;
        let mut view = results_rect.to_renderer(&mut render);
        let view_y = view.dimensions().y;
        for (i, match_) in self.cur_matches.iter().enumerate().take(view_y) {
            view.print(
                render::Coord {
                    x: 0,
                    y: view_y.saturating_sub(1).saturating_sub(i),
                },
                &format!("{}", match_.display()),
                if i == 0 {
                    style_selected
                } else {
                    style_default
                },
            );
        }

        let mut status_view = status_rect.to_renderer(&mut render);
        status_view.print(
            render::Coord { x: 0, y: 0 },
            &format!("find: {}", self.match_str),
            style_default,
        );
    }
}
