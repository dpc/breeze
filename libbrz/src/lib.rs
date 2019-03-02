#![allow(dead_code)]

pub mod buffer;
pub mod coord;
pub mod idx;
pub mod mode;
pub mod selection;

pub mod prelude;

use crate::buffer::Buffer;
use crate::mode::Mode;
use prelude::*;
use ropey::Rope;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// Backspace.
    Backspace,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// Function keys.
    ///
    /// Only function keys 1 through 12 are supported.
    F(u8),
    /// Normal character.
    Char(char),
    /// Alt modified character.
    Alt(char),
    /// Ctrl modified character.
    ///
    /// Note that certain keys may not be modifiable with `ctrl`, due to limitations of terminals.
    Ctrl(char),
    /// Null byte.
    Null,
    /// Esc key.
    Esc,

    #[doc(hidden)]
    __IsNotComplete,
}

/// The editor state
#[derive(Clone)]
pub struct State {
    quit: bool,
    mode: Mode,
    buffer: Buffer,
    buffer_history: Vec<Buffer>,
    buffer_history_undo_i: Option<usize>,
    yanked: Vec<Rope>,

    cmd: String,
    msg: Option<String>,
}

impl State {
    fn maybe_commit_undo_point(mut self, prev_buf: &Buffer) -> Self {
        if self.buffer_history.last().map(|b| &b.text) != Some(&self.buffer.text) {
            self.buffer_history.push(prev_buf.clone());
        }
        self.buffer_history_undo_i = None;
        self
    }

    pub fn cmd_string(&self) -> Option<String> {
        if let Some(ref msg) = self.msg {
            Some(msg.to_owned())
        } else if let Mode::Command = self.mode {
            Some(format!(":{}", self.cmd))
        } else {
            None
        }
    }

    fn commit_undo_point(mut self) -> Self {
        if self.buffer_history.last() != Some(&self.buffer) {
            self.buffer_history.push(self.buffer.clone());
        }
        self.buffer_history_undo_i = None;
        self
    }

    pub fn open_buffer(&mut self, buffer: Buffer) {
        self.buffer = buffer;
    }

    pub fn is_finished(&self) -> bool {
        self.quit
    }

    pub fn handle(mut self, key: Key) -> Self {
        self.msg = None;
        self.mode.handle(self.clone(), key)
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn mode_name(&self) -> &str {
        self.mode.name()
    }

    pub fn mode_num_prefix_str(&self) -> String {
        self.mode.num_prefix_str()
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            quit: false,
            mode: Mode::default(),
            buffer: default(),
            buffer_history: vec![],
            buffer_history_undo_i: None,
            yanked: vec![],
            cmd: "".into(),
            msg: None,
        }
    }
}
