use crate::mode;
use crate::Idx;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

use crate::Key;

use super::*;

macro_rules! action {
    ($name:ident, $help:expr, ($state:ident) $body:block) => {
        struct $name;

        impl Action for $name {
            fn help(&self) -> &str {
                $help
            }

            fn execute(&self, $state: &mut State) {
                $body
            }
        }
    };
}

action!(EnterCommandMode, "command mode", (state) {
    state.set_mode(mode::Command::new());
});

action!(EnterInsertMode, "insert mode", (state) {
    state.set_mode(mode::Insert);
});

action!(MoveDownPage, "move down page", (state) {
    state.cur_buffer_mut().move_cursor_down(25);
});

action!(ExtendDownPage, "extend down page", (state) {
    state.cur_buffer_mut().extend_cursor_down(25);
});

action!(LineAppend, "append to line", (state) {
    state
        .cur_buffer_mut()
        .extend_cursor(Idx::forward_to_line_end);
    state.set_mode(mode::Insert);
});
action!(EnterOpenMode, "open mode", (state) {
    state.set_mode(mode::Find::default());
});

action!(MoveUpPage, "move up page", (state) {
    state.cur_buffer_mut().move_cursor_up(25);
});
action!(ExtendUpPage, "extend up page", (state) {
    state.cur_buffer_mut().extend_cursor_up(25);
});

macro_rules! action_key {
    ($m:ident, $k:expr, $name:ident) => {
        $m.insert(NaturalyOrderedKey(Key::Char($k)), Box::new($name) as Box<_>);
    };
}

macro_rules! action_ctrl_key {
    ($m:ident, $k:expr, $name:ident) => {
        $m.insert(NaturalyOrderedKey(Key::Ctrl($k)), Box::new($name) as Box<_>);
    };
}
pub fn all_actions() -> &'static super::Map {
    static INSTANCE: OnceCell<super::Map> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = BTreeMap::new();

        action_key!(m, 'i', EnterInsertMode);
        action_key!(m, ':', EnterCommandMode);
        action_key!(m, 'A', LineAppend);
        action_ctrl_key!(m, 'p', EnterOpenMode);
        action_ctrl_key!(m, 'u', MoveUpPage);
        action_ctrl_key!(m, 'U', ExtendUpPage);
        action_ctrl_key!(m, 'd', MoveDownPage);
        action_ctrl_key!(m, 'D', ExtendDownPage);
        m
    })
}
