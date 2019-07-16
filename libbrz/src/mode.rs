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
}
