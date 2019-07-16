use crate::mode;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

use crate::Key;

use super::*;

pub fn all_actions() -> &'static super::Map {
    static INSTANCE: OnceCell<super::Map> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = BTreeMap::new();

        m.insert(Key::Char('i'), Box::new(EnterInsertMode) as Box<_>);
        m
    })
}

struct EnterInsertMode;

impl Action for EnterInsertMode {
    fn help(&self) -> &str {
        "enter insert mode"
    }

    fn execute(&self, state: &mut State) {
        state.set_mode(mode::Insert);
    }
}
