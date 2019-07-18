use crate::action;
use crate::idx::*;
use crate::state::State;
use crate::Key;
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

pub use crate::render::{self, Renderer};

// TODO: mode should render itself, ha!
pub trait Mode {
    fn name(&self) -> &str;
    fn cmd_string(&self) -> Option<String> {
        None
    }
    fn available_actions(&self) -> &action::Map {
        action::no_actions()
    }

    fn handle(&mut self, state: &mut State, key: Key);

    fn render(&self, state: &State, render: &mut dyn Renderer) {
        default_render(self, state, render)
    }
}

fn default_render(mode: &(impl Mode + ?Sized), state: &State, mut render: &mut dyn Renderer) {
    let total_rect = render.rect();
    let (buffer_rect, status_rect) = total_rect.split_horizontaly_at(-1);
    state.render_buffer(&mut buffer_rect.to_renderer(&mut render));
    let mut status_view = status_rect.to_renderer(&mut render);
    status_view.print(
        render::Coord { x: 0, y: 0 },
        mode.name(),
        render::Style::default(),
    );
    unimplemented!();
}
