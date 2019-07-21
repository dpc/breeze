use crate::buffer::Buffer;
use crate::buffer::VisualSelection;
use crate::mode::{self, Mode};
use crate::Idx;
use crate::Key;
use default::default;
use ropey::Rope;

use crate::render::{self, Renderer};
use crate::{buffer, position};
use std::cmp::min;
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
            let new_i = min(undo_i.saturating_add(times), self.buffer_history.len() - 1);
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
    cur_buffer_i: Option<usize>,
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
        self.cur_buffer_state_mut_opt()
            .map(|b| b.maybe_commit_undo_point());
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
            self.cur_buffer_i = Some(found);
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

        self.cur_buffer_i = Some(entry.key());
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
        if let Some(cur_buffer_i) = self.cur_buffer_i {
            (self.write_handler)(path, &self.buffers[cur_buffer_i].buffer.text)?;
        }

        Ok(())
    }

    pub fn delete_buffer(&mut self) {
        if let Some(cur_buffer_i) = self.cur_buffer_i {
            self.buffers.remove(cur_buffer_i);
            self.buffer_next()
        }
    }

    pub fn buffer_next(&mut self) {
        if !self.buffers.is_empty() {
            let mut cur_buffer_i = self.cur_buffer_i.expect("cur_buffer_i set");
            loop {
                cur_buffer_i += 1;
                cur_buffer_i %= self.buffers.capacity();
                if self.buffers.contains(cur_buffer_i) {
                    self.cur_buffer_i = Some(cur_buffer_i);
                    break;
                }
            }
        } else {
            self.cur_buffer_i = None;
        }
    }

    pub fn buffer_prev(&mut self) {
        if !self.buffers.is_empty() {
            let mut cur_buffer_i = self.cur_buffer_i.expect("cur_buffer_i set");
            loop {
                if cur_buffer_i == 0 {
                    cur_buffer_i = self.buffers.capacity() - 1;
                } else {
                    cur_buffer_i -= 1;
                }
                if self.buffers.contains(cur_buffer_i) {
                    self.cur_buffer_i = Some(cur_buffer_i);
                    break;
                }
            }
        } else {
            self.cur_buffer_i = None;
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

    pub fn cur_buffer_opt(&self) -> Option<&Buffer> {
        self.cur_buffer_i.map(|i| &self.buffers[i].buffer)
    }

    pub fn cur_buffer_mut_opt(&mut self) -> Option<&mut Buffer> {
        self.cur_buffer_i.map(move |i| &mut self.buffers[i].buffer)
    }

    pub fn cur_buffer(&self) -> &Buffer {
        self.cur_buffer_opt().expect("cur_buffer set")
    }

    pub fn cur_buffer_mut(&mut self) -> &mut Buffer {
        self.cur_buffer_mut_opt().expect("cur_buffer set")
    }

    pub fn cur_buffer_state_opt(&self) -> Option<&BufferState> {
        self.cur_buffer_i.map(|i| &self.buffers[i])
    }

    pub fn cur_buffer_state_mut_opt(&mut self) -> Option<&mut BufferState> {
        self.cur_buffer_i.map(move |i| &mut self.buffers[i])
    }

    pub fn cur_buffer_state(&self) -> &BufferState {
        self.cur_buffer_state_opt().expect("cur_buffer set")
    }

    pub fn cur_buffer_state_mut(&mut self) -> &mut BufferState {
        self.cur_buffer_state_mut_opt().expect("cur_buffer set")
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

    pub fn render_buffer(&self, mut render: &mut dyn Renderer) {
        if self.cur_buffer_opt().is_none() {
            self.render_splash(render);
            return;
        }

        let buffer = self.cur_buffer();
        let dims = render.dimensions();

        let window_height = dims.y;
        let window_margin = window_height / 4;
        let cursor_coord = buffer.cursor_coord();
        let first_line_that_must_be_visible = cursor_coord.line.saturating_sub(window_margin);
        let last_line_that_must_by_visible = min(
            cursor_coord.line.saturating_add(window_margin),
            buffer.lines(),
        );
        debug_assert!(first_line_that_must_be_visible <= last_line_that_must_by_visible);

        let mut line_offset = buffer.view_line_offset.borrow_mut();

        if first_line_that_must_be_visible < *line_offset {
            *line_offset = first_line_that_must_be_visible;
        }
        if line_offset.saturating_add(window_height) < last_line_that_must_by_visible {
            *line_offset = last_line_that_must_by_visible.saturating_sub(window_height);
        }

        let start_line = min(*line_offset, buffer.lines().saturating_sub(window_height));
        let end_line = min(start_line.saturating_add(window_height), buffer.lines());

        drop(line_offset);
        let line_nums_width = buffer.lines().to_string().len() + 1;
        let (line_nums_rect, content_rect) = render
            .dimensions_rect()
            .split_verticaly_at(line_nums_width as isize);
        self.render_line_nums(
            &mut line_nums_rect.to_renderer(&mut render),
            start_line,
            end_line,
        );
        self.render_content(&mut content_rect.to_renderer(&mut render), start_line);
    }

    pub fn render_line_nums(&self, render: &mut dyn Renderer, start_line: usize, end_line: usize) {
        let width = render.dimensions().x;
        let style = render.color_map().line_num;
        for line in start_line..end_line {
            let line_str = format!("{} ", line.to_string());
            render.print(
                render::Coord {
                    x: width - line_str.len(),
                    y: line - start_line,
                },
                &line_str,
                style,
            );
        }
    }

    pub fn render_content(&self, render: &mut dyn Renderer, start_line: usize) {
        let buffer = self.cur_buffer();
        let window_dims = render.dimensions();
        let cursor_coord = buffer.cursor_coord();

        let color_map = { render.color_map().to_owned() };

        let mut cur_ch_idx = position::Position {
            line: start_line,
            column: 0,
        }
        .to_idx(&buffer.text)
        .0;

        let mut cur_visual_coord = render::Coord { x: 0, y: 0 };

        loop {
            if window_dims.y <= cur_visual_coord.y {
                break;
            }

            if buffer.text.len_chars() <= cur_ch_idx {
                break;
            }

            let ch = buffer.text.char(cur_ch_idx);

            let visual_selection = buffer.idx_selection_type(Idx(cur_ch_idx));

            let style = color_map.default;

            let (visual_ch, visual_ch_width, special) = match ch {
                '\n' => {
                    if visual_selection != VisualSelection::None {
                        (Some('↩'), 1, true) // alternatives: ⤶  🡿
                    } else {
                        (None, 0, true)
                    }
                }
                '\t' => (
                    Some('.'),
                    buffer::distance_to_next_tabstop(
                        Idx(cur_ch_idx).to_position(&buffer.text).column,
                        4,
                    ),
                    false,
                ),
                ch => (Some(ch), 1, false),
            };

            let style = if special & visual_ch.is_some() {
                style.paintover(color_map.special)
            } else {
                style
            };

            let style = match visual_selection {
                VisualSelection::DirectionMarker => style.paintover(color_map.direction_marker),
                VisualSelection::Selection => style.paintover(color_map.selection),
                VisualSelection::None => style,
            };

            if ch == '\n' {
                if let Some(visual_ch) = visual_ch {
                    render.put(cur_visual_coord, visual_ch, style);
                }
                cur_visual_coord.x = 0;
                cur_visual_coord.y += 1;
            } else {
                for _ in 0..visual_ch_width {
                    if let Some(visual_ch) = visual_ch {
                        render.put(cur_visual_coord, visual_ch, style);
                    }
                    cur_visual_coord.x += 1;
                }
            }
            cur_ch_idx += 1;
        }

        render.set_cursor(Some(render::Coord {
            y: cursor_coord.line.saturating_sub(start_line),
            x: cursor_coord.column,
        }));
    }

    pub fn render_splash(&self, render: &mut dyn Renderer) {
        let center = render.dimensions().center();
        let style = render.color_map().default;
        render.print_centered(
            center.sub_y(1),
            &format!("Breeze v{}", env!("CARGO_PKG_VERSION")),
            style,
        );
        render.print_centered(center.add_y(1), "by Dawid Ciężarkiewicz", style);
        render.set_cursor(None);
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
        State {
            quit: false,
            mode: Some(Box::new(mode::Normal::default())),
            yanked: vec![],
            find_result: None,
            msg: None,

            buffers: Slab::new(),
            cur_buffer_i: None,

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
        }
    }
}
