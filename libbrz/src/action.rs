pub use super::Mode;
pub use super::State;

pub mod normal;

pub use self::normal::all_actions;
use crate::Key;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

pub type Map = BTreeMap<Key, Box<Action + Send + Sync + 'static>>;

pub fn no_actions() -> &'static Map {
    static INSTANCE: OnceCell<Map> = OnceCell::new();
    INSTANCE.get_or_init(|| BTreeMap::new())
}
pub trait Action {
    fn help(&self) -> &str;

    fn execute(&self, state: &mut State);
}
