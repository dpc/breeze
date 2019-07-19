use crate::buffer::Buffer;
use crate::mode::{self, Mode};
use crate::Key;
use default::default;
use ropey::Rope;

use crate::render::Renderer;
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
    pub(crate) fn maybe_commit_undo_point(&mut self) {
        if let Some(restored_i) = self.buffer_history_undo_i {
            if self.buffer_history[restored_i].text != self.buffer.text {
                // if we started editing and content changed after restoring from undo,
                // we reset the undo point and start appending commit new undo points
                self.buffer_history_undo_i = None;

                let new_buffer = self.buffer.clone();
                self.buffer = self.buffer_history[restored_i].clone();
                self.maybe_commit_undo_point();
                self.buffer = new_buffer;
                self.maybe_commit_undo_point();
            } else if self.buffer_history[restored_i].selection != self.buffer.selection {
                // XXX: TODO: We're editing history... :/ ... seems bad; does it give better UX?
                self.buffer_history[restored_i].selection = self.buffer.selection.clone();
            }
        } else if let Some(last) = self.buffer_history.last_mut() {
            if last.text != self.buffer.text {
                // if buffer changed, we make it a new undo point
                self.buffer_history.push(self.buffer.clone());
            } else if last.selection != self.buffer.selection {
                // if only the selection changed, we previous undo point,
                // so undo always jumps to last cursor/selectin position from
                // before the edit
                last.selection = self.buffer.selection.clone();
            }
        } else {
            self.buffer_history.push(self.buffer.clone());
        }
    }

    pub(crate) fn undo(&mut self, times: usize) {
        let i = if let Some(restored_i) = self.buffer_history_undo_i {
            restored_i.saturating_sub(times)
        } else {
            self.maybe_commit_undo_point(); // commit to unify
            self.buffer_history
                .len()
                .saturating_sub(1)
                .saturating_sub(times)
        };

        self.buffer_history_undo_i = Some(i);
        self.buffer = self.buffer_history[i].clone();
    }

    pub(crate) fn redo(&mut self, times: usize) {
        if let Some(undo_i) = self.buffer_history_undo_i.as_mut() {
            let new_i = std::cmp::min(undo_i.saturating_add(times), self.buffer_history.len() - 1);
            *undo_i = new_i;
            self.buffer = self.buffer_history[new_i].clone();
        }
    }
}

/// The editor state
pub struct State {
    pub(crate) quit: bool,
    mode: Option<Box<dyn Mode + 'static>>,
    pub(crate) yanked: Vec<Rope>,

    pub(crate) find_result: Option<PathBuf>,
    pub(crate) msg: Option<String>,

    pub(crate) read_handler: Arc<Fn(&Path) -> io::Result<Rope>>,
    pub(crate) write_handler: Arc<Fn(&Path, &Rope) -> io::Result<()>>,
    pub(crate) find_handler: Arc<Fn(&str) -> io::Result<Vec<PathBuf>>>,

    buffers: Slab<BufferState>,
    cur_buffer_i: usize,
}

impl State {
    pub fn cmd_string(&self) -> Option<String> {
        if let Some(ref msg) = self.msg {
            Some(msg.to_owned())
        } else {
            self.mode.as_ref().expect("mode set").cmd_string()
        }
    }

    pub(crate) fn set_mode(&mut self, mode: impl Mode + 'static) {
        self.cur_buffer_state_mut().maybe_commit_undo_point();
        self.mode = Some(Box::new(mode) as Box<dyn Mode>);
    }

    pub fn get_mode(&self) -> &(dyn Mode + 'static) {
        &**self.mode.as_ref().unwrap()
    }

    /*
    pub(crate) fn get_mode(&mut self) -> &mut Mode {
        &mut self.mode
    }
    */

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

    pub fn write_buffer(&mut self, path: Option<PathBuf>) {
        if let Some(path) = path.or_else(|| self.cur_buffer_state().path.clone()) {
            match self.try_write_buffer(&path) {
                Ok(()) => {
                    self.cur_buffer_state_mut().path = Some(path);
                }
                Err(e) => {
                    self.msg = Some(format!("{}", e));
                }
            }
        } else {
            self.msg = Some("No path given".to_string());
        }
    }

    fn try_write_buffer(&self, path: &Path) -> io::Result<()> {
        (self.write_handler)(path, &self.buffers[self.cur_buffer_i].buffer.text)
    }

    pub fn open_scratch_buffer(&mut self) {
        self.buffers.insert(default());
    }

    pub fn delete_buffer(&mut self) {
        self.buffers.remove(self.cur_buffer_i);
        if self.buffers.is_empty() {
            self.open_scratch_buffer();
        }
        self.buffer_next()
    }

    pub fn buffer_next(&mut self) {
        loop {
            self.cur_buffer_i += 1;
            self.cur_buffer_i %= self.buffers.capacity();
            if self.buffers.contains(self.cur_buffer_i) {
                break;
            }
        }
    }

    pub fn buffer_prev(&mut self) {
        loop {
            if self.cur_buffer_i == 0 {
                self.cur_buffer_i = self.buffers.capacity() - 1;
            } else {
                self.cur_buffer_i -= 1;
            }
            if self.buffers.contains(self.cur_buffer_i) {
                break;
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.quit
    }

    pub fn handle_key(&mut self, key: Key) {
        self.msg = None;
        let mut mode = self.mode.take().expect("mode set");

        mode.handle(self, key);

        if self.mode.is_none() {
            self.mode = Some(mode)
        }
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
        self.mode.as_ref().expect("mode set").name()
    }

    pub fn register_read_handler(&mut self, f: impl Fn(&Path) -> io::Result<Rope> + 'static) {
        self.read_handler = Arc::new(f);
    }

    pub fn register_write_handler(&mut self, f: impl Fn(&Path, &Rope) -> io::Result<()> + 'static) {
        self.write_handler = Arc::new(f);
    }

    pub fn register_find_handler(
        &mut self,
        f: impl Fn(&str) -> io::Result<Vec<PathBuf>> + 'static,
    ) {
        self.find_handler = Arc::new(f);
    }

    pub fn render(&self, render: &mut dyn Renderer) {
        self.mode.as_ref().expect("mode set").render(self, render);
    }

    pub fn render_buffer(&self, render: &mut dyn Renderer) {
        let center = render.dimensions().center();
        let style = render.color_map().default_style();
        render.print(center, "HELLO WORLD", style);
        render.set_cursor(Some(center));
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
        let mut s = State {
            quit: false,
            mode: Some(Box::new(mode::Normal::default())),
            yanked: vec![],
            find_result: None,
            msg: None,

            buffers: Slab::new(),
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
            find_handler: Arc::new(|_str| {
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "handler not registered",
                ))
            }),
        };

        s.open_scratch_buffer();
        s.buffer_next();

        s
    }
}
