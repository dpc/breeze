use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::Key;
use default::default;
use ropey::Rope;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slab::Slab;

#[derive(Clone)]
pub struct BufferState {
    pub(crate) buffer: Buffer,
    pub(crate) buffer_history: Vec<Buffer>,
    pub(crate) buffer_history_undo_i: Option<usize>,

    path: Option<PathBuf>,
}

impl BufferState {
    pub(crate) fn maybe_commit_undo_point(&mut self, prev_buf: &Buffer) {
        if self.buffer_history.last().map(|b| &b.text) != Some(&self.buffer.text) {
            self.buffer_history.push(prev_buf.clone());
        }
        self.buffer_history_undo_i = None;
    }
    pub(crate) fn commit_undo_point(&mut self) {
        if self.buffer_history.last() != Some(&self.buffer) {
            self.buffer_history.push(self.buffer.clone());
        }
        self.buffer_history_undo_i = None;
    }
}

/// The editor state
#[derive(Clone)]
pub struct State {
    pub(crate) quit: bool,
    mode: Mode,
    pub(crate) yanked: Vec<Rope>,

    pub(crate) cmd: String,
    pub(crate) msg: Option<String>,

    pub(crate) read_handler: Arc<Fn(&Path) -> io::Result<Rope>>,
    pub(crate) write_handler: Arc<Fn(&Path, &Rope) -> io::Result<()>>,

    buffers: Slab<BufferState>,
    cur_buffer_i: usize,
}

impl State {
    pub fn cmd_string(&self) -> Option<String> {
        if let Some(ref msg) = self.msg {
            Some(msg.to_owned())
        } else if let Mode::Command = self.mode {
            Some(format!(":{}", self.cmd))
        } else {
            None
        }
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub(crate) fn get_mode(&mut self) -> &mut Mode {
        &mut self.mode
    }

    pub fn open_buffer(&mut self, path: PathBuf) {
        let mut found = None;

        for (i, buffer_state) in self.buffers.iter() {
            if buffer_state.path.as_ref() == Some(&path) {
                found = Some(i);
                break;
            }
        }

        if let Some(found) = found {
            self.cur_buffer_i = found;
            return;
        }

        let rope = match (self.read_handler)(&path) {
            Err(e) => {
                self.msg = Some(format!("{}", e));
                return;
            }
            Ok(rope) => rope,
        };

        let entry = self.buffers.vacant_entry();

        self.cur_buffer_i = entry.key();
        entry.insert(BufferState {
            path: Some(path),
            buffer: Buffer::from_text(rope),
            ..default()
        });
    }

    pub fn is_finished(&self) -> bool {
        self.quit
    }

    pub fn handle(mut self, key: Key) -> Self {
        self.msg = None;
        self.mode.handle(self.clone(), key)
    }

    pub fn cur_buffer(&self) -> &Buffer {
        &self.buffers[self.cur_buffer_i].buffer
    }

    pub fn cur_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.cur_buffer_i].buffer
    }

    pub fn cur_buffer_state(&self) -> &BufferState {
        &self.buffers[self.cur_buffer_i]
    }

    pub fn cur_buffer_state_mut(&mut self) -> &mut BufferState {
        &mut self.buffers[self.cur_buffer_i]
    }
    pub fn mode_name(&self) -> &str {
        self.mode.name()
    }

    pub fn mode_num_prefix_str(&self) -> String {
        self.mode.num_prefix_str()
    }

    pub fn register_read_handler(&mut self, f: impl Fn(&Path) -> io::Result<Rope> + 'static) {
        self.read_handler = Arc::new(f);
    }

    pub fn register_write_handler(&mut self, f: impl Fn(&Path, &Rope) -> io::Result<()> + 'static) {
        self.write_handler = Arc::new(f);
    }
}

impl Default for BufferState {
    fn default() -> Self {
        Self {
            buffer: default(),
            buffer_history: vec![],
            buffer_history_undo_i: None,
            path: None,
        }
    }
}
impl Default for State {
    fn default() -> Self {
        let mut buffers = Slab::new();
        buffers.insert(default());

        State {
            quit: false,
            mode: Mode::default(),
            yanked: vec![],
            cmd: "".into(),
            msg: None,

            buffers,
            cur_buffer_i: 0,

            read_handler: Arc::new(|_path| {
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "handler not registered",
                ))
            }),
            write_handler: Arc::new(|_path, _rope| {
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "handler not registered",
                ))
            }),
        }
    }
}
