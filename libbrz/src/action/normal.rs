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

macro_rules! actions {
    ($m:ident) => {};
    ($m:ident,) => {};
    ($m:ident, {  $name:ident, $help:expr, ($state:ident) $body:block }, $($rest:tt)*) => {
        action!($name, $help, ($state) $body);
        $m.insert(stringify!($name), Box::new($name) as Box<_>);
        actions!($m, $($rest)*);
    };
}

pub fn actions() -> &'static super::ActionsById {
    static INSTANCE: OnceCell<super::ActionsById> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = BTreeMap::new();

        actions!(
            m,

        { Command, "command mode", (state) {
            state.set_mode(mode::Command::new());
        }},

        { Insert, "insert mode", (state) {
            state.set_mode(mode::Insert::new_normal());
        }},

        { InsertExtend , "insert extend mode", (state) {
            state.set_mode(mode::Insert::new_extend());
        }},

        { MoveDownPage, "move down page", (state) {
            state.cur_buffer_mut().move_cursor_down(25);
        }},

        { ExtendDownPage, "extend down page", (state) {
            state.cur_buffer_mut().extend_cursor_down(25);
        }},

        { LineAppend, "append to line", (state) {
            state
                .cur_buffer_mut()
                .extend_cursor(Idx::forward_to_line_end);
            state.set_mode(mode::Insert::new_normal());
        }},

        { LineAppendExtend, "append to line (extend)", (state) {
            state
                .cur_buffer_mut()
                .extend_cursor(Idx::forward_to_line_end);
            state.set_mode(mode::Insert::new_extend());
        }},
        { OpenFile, "open mode", (state) {
            state.set_mode(mode::Find::default());
        }},

        { MoveUpPage, "move up page", (state) {
            state.cur_buffer_mut().move_cursor_up(25);
        }},
        { ExtendUpPage, "extend up page", (state) {
            state.cur_buffer_mut().extend_cursor_up(25);
        }},
        );
        m
    })
}

macro_rules! key_mappings {
    ($m:ident) => {};
    ($m:ident,) => {};
    ($m:ident, { $k:ident, $name:ident }, $($rest:tt)*) => {
        $m.insert(NaturalyOrderedKey(Key::Char(stringify!($k).chars().next().unwrap())), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
    ($m:ident, { $k:expr, $name:ident }, $($rest:tt)*) => {
        $m.insert(NaturalyOrderedKey(Key::Char($k)), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
    ($m:ident, { c_$k:ident, $name:ident }, $($rest:tt)*) => {
        $m.insert(NaturalyOrderedKey(Key::Ctrl(stringify!($k).chars().next().unwrap())), stringify!($name));
        key_mappings!($m, $($rest)*);
    };
}

pub fn default_key_mappings() -> &'static super::KeyMappings {
    static INSTANCE: OnceCell<super::KeyMappings> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = BTreeMap::new();

        key_mappings!(
            m,
            { i, Insert },
            { I, InsertExtend },
            { ':', Command },
            { a, LineAppend },
            { c_p, OpenFile },
            { c_u, MoveUpPage },
            { c_d, MoveDownPage },
            { c_U, ExtendUpPage },
            { c_D, ExtendDownPage },
        );
        m
    })
}
