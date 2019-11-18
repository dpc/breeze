pub use super::Mode;
pub use super::State;

pub mod normal;

pub use self::normal::default_key_mappings;
use crate::NaturalyOrderedKey;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

pub type ActionRef<'a> = &'a (dyn Action + Send + Sync + 'static);
pub type ActionByKey<'a> = (super::Key, ActionRef<'a>);
pub type ActionsById = BTreeMap<&'static str, Box<dyn Action + Send + Sync + 'static>>;
pub type KeyMappings = BTreeMap<NaturalyOrderedKey, &'static str>;

pub fn empty_actions_by_id() -> &'static ActionsById {
    static INSTANCE: OnceCell<ActionsById> = OnceCell::new();
    INSTANCE.get_or_init(BTreeMap::new)
}

pub fn empty_key_mappings() -> &'static KeyMappings {
    static INSTANCE: OnceCell<KeyMappings> = OnceCell::new();
    INSTANCE.get_or_init(BTreeMap::new)
}

pub trait Action {
    fn help(&self) -> &str;

    fn execute(&self, state: &mut State);
}
