use crate::action;
use crate::idx::*;
use crate::state::State;
use crate::Key;
use std::cmp::min;
use std::path::PathBuf;

mod command;
mod find;
mod goto;
mod insert;
mod normal;

pub use self::command::Command;
pub use self::find::Find;
pub use self::goto::Goto;
pub use self::insert::Insert;
pub use self::normal::Normal;

pub use crate::render::{self, Coord, Rect, Renderer};

// TODO: mode should render itself, ha!
pub trait Mode {
    fn name(&self) -> &str;
    fn name4(&self) -> &str {
        &self.name()[..4]
    }
    fn cmd_string(&self) -> Option<String> {
        None
    }

    fn action_mappings<'s>(&'s self) -> Box<dyn Iterator<Item = action::ActionByKey<'s>> + 's> {
        let actions = self.actions();
        Box::new(
            self.key_mappings()
                .into_iter()
                .map(move |(key, action)| (key.0, &**actions.get(action).expect("action present"))),
        )
    }

    fn key_mappings(&self) -> &action::KeyMappings {
        action::empty_key_mappings()
    }

    fn actions(&self) -> &action::ActionsById {
        action::empty_actions_by_id()
    }

    fn action_by_key(&self, key: super::Key) -> Option<action::ActionRef> {
        self.key_mappings()
            .get(&super::NaturalyOrderedKey(key))
            .and_then(|name| self.actions().get(name))
            .map(|boxed| &**boxed)
    }

    fn on_enter(&mut self, _state: &State) {}

    fn handle(&mut self, state: &mut State, key: Key);

    fn render(&self, state: &State, render: &mut dyn Renderer) {
        let _ = default_render(self, state, render);
    }
}

fn default_render_split_status_rect(render: &mut dyn Renderer) -> (Rect, Rect) {
    let total_rect = render.dimensions_rect();
    total_rect.split_horizontaly_at(-1)
}

fn default_render_status(
    mode: &(impl Mode + ?Sized),
    mut render: &mut dyn Renderer,
    status_rect: Rect,
) {
    let style = render.color_map().default;
    let mut status_view = status_rect.to_renderer(&mut render);
    status_view.print(
        render::Coord {
            x: status_rect.dimensions.x.saturating_sub(4),
            y: 0,
        },
        mode.name4(),
        style,
    );
}

fn default_render(
    mode: &(impl Mode + ?Sized),
    state: &State,
    mut render: &mut dyn Renderer,
) -> (Rect, Rect) {
    let (buffer_rect, status_rect) = default_render_split_status_rect(render);
    state.render_buffer(&mut buffer_rect.to_renderer(&mut render));
    if state.cur_buffer_opt().is_some() {
        default_render_available_actions(mode, state, render, buffer_rect);
    }

    default_render_status(mode, render, status_rect);

    (buffer_rect, status_rect)
}

fn default_render_available_actions(
    mode: &(impl Mode + ?Sized),
    state: &State,
    mut render: &mut dyn Renderer,
    buffer_rect: Rect,
) {
    let actions = mode.key_mappings();

    if actions.is_empty() {
        return;
    }
    let style = render.color_map().actions;

    let height = min(buffer_rect.dimensions.y / 2, actions.len());
    let width = buffer_rect.dimensions.x / 3;

    let cursor_x = state.last_visual_cursor_coord.borrow().expect("set").x;

    let rect = if cursor_x < buffer_rect.dimensions.x / 2 {
        buffer_rect
            .split_horizontaly_at(-((height + 1) as isize))
            .1
            .split_verticaly_at(-(width as isize))
            .1
    } else {
        buffer_rect
            .split_horizontaly_at(-((height + 1) as isize))
            .1
            .split_verticaly_at(width as isize)
            .0
    };

    let mut view = rect.to_renderer(&mut render);
    view.fill(view.dimensions_rect(), ' ', style);
    view.print_centered(
        Coord {
            y: 0,
            x: rect.dimensions.x / 2,
        },
        "available commands",
        style,
    );
    for (i, action) in mode.action_mappings().enumerate().take(height) {
        view.print(
            render::Coord { x: 0, y: i + 1 },
            &format!("{:>3} {}", (action.0), action.1.help()),
            style,
        );
    }
}
