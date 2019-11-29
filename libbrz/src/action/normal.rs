use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

use crate::mode;
use crate::Idx;
use crate::Key;

use crate::{action, actions, key_mappings};

pub fn actions() -> &'static super::ActionsById {
    static INSTANCE: OnceCell<super::ActionsById> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = BTreeMap::new();

        actions!(
            m,

            Command, "command mode", (state) {
                state.set_mode(mode::Command::new());
            },

            Insert, "insert mode", (state) {
                state.set_mode(mode::Insert::new_normal());
            },

            InsertExtend , "insert extend mode", (state) {
                state.set_mode(mode::Insert::new_extend());
            },

            MoveDownPage, "move down page", (state) {
                state.cur_buffer_mut().move_cursor_down(25);
            },

            ExtendDownPage, "extend down page", (state) {
                state.cur_buffer_mut().extend_cursor_down(25);
            },

            LineAppend, "append to line", (state) {
                state
                    .cur_buffer_mut()
                    .extend_cursor(Idx::forward_to_line_end);
                state.set_mode(mode::Insert::new_normal());
            },

            LineAppendExtend, "append to line (extend)", (state) {
                state
                    .cur_buffer_mut()
                    .extend_cursor(Idx::forward_to_line_end);
                state.set_mode(mode::Insert::new_extend());
            },

            OpenFile, "open mode", (state) {
                state.set_mode(mode::Find::default());
            },

            MoveUpPage, "move up page", (state) {
                state.cur_buffer_mut().move_cursor_up(25);
            },

            ExtendUpPage, "extend up page", (state) {
                state.cur_buffer_mut().extend_cursor_up(25);
            },

            IndentRight, "indent right", (state) {
                let times = state.take_num_prefix();
                state.cur_buffer_mut().increase_indent(times);
            },

            IndentLeft, "indent left", (state) {
                let times = state.take_num_prefix();
                state.cur_buffer_mut().decrease_indent(times);
            },


            OpenLine, "open line", (state) {
                state.cur_buffer_mut().open();
                state.set_mode(mode::Insert::new_normal());
            },

            SelectInnerSurrounding, "select inner surrounding", (state) {
                state.cur_buffer_mut().select_inner_surrounding();
            },
            ExpandInnerSurrounding, "expand inner surrounding", (state) {
                state.cur_buffer_mut().select_inner_surrounding();
            },
        );
        m
    })
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
            { A, LineAppendExtend },
            { c p, OpenFile },
            { c u, MoveUpPage },
            { c d, MoveDownPage },
            { c U, ExtendUpPage },
            { c D, ExtendDownPage },
            { '>', IndentRight },
            { '<', IndentLeft },
            { 'o', OpenLine },
            { a i, SelectInnerSurrounding },
            { a I, ExpandInnerSurrounding },
        );
        m
    })
}
