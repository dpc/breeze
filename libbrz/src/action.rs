pub use super::Mode;
pub use super::State;

pub mod normal;

pub use self::normal::default_key_mappings;
use crate::NaturalyOrderedKey;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

#[macro_export]
macro_rules! action {
    ($name:ident, $help:expr, ($state:ident) $body:block) => {
        struct $name;

        impl $crate::action::Action for $name {
            fn help(&self) -> &str {
                $help
            }

            fn execute(&self, $state: &mut $crate::State) {
                $body
            }
        }
    };
}

#[macro_export]
macro_rules! actions {
    ($m:ident) => {};
    ($m:ident,) => {};
    ($m:ident,  $name:ident, $help:expr, ($state:ident) $body:block , $($rest:tt)*) => {
        action!($name, $help, ($state) $body);
        $m.insert(stringify!($name), Box::new($name) as Box<_>);
        actions!($m, $($rest)*);
    };
}

#[macro_export]
macro_rules! key_mappings {
    ($m:ident) => {};
    ($m:ident,) => {};
    ($m:ident, { $k:ident, $name:ident }, $($rest:tt)*) => {
        $m.insert($crate::action::NaturalyOrderedKey(Key::Char(stringify!($k).chars().next().unwrap())), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
    ($m:ident, { $k:expr, $name:ident }, $($rest:tt)*) => {
        $m.insert($crate::action::NaturalyOrderedKey(Key::Char($k)), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
    ($m:ident, { c_$k:ident, $name:ident }, $($rest:tt)*) => {
        $m.insert(NaturalyOrderedKey(Key::Ctrl(stringify!($k).chars().next().unwrap())), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
}

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
