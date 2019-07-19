use std::path::Path;

use std::io::{self, Write};
use structopt::StructOpt;
use termion::color;
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;
use termion::style;

use libbrz::render;

use ropey::Rope;
use std;
use std::process;

mod opts;

use libbrz::{prelude::*, state::State};

fn termion_to_brz_key(key: termion::event::Key) -> libbrz::Key {
    match key {
        termion::event::Key::Backspace => libbrz::Key::Backspace,
        termion::event::Key::Left => libbrz::Key::Left,
        termion::event::Key::Up => libbrz::Key::Up,
        termion::event::Key::Right => libbrz::Key::Right,
        termion::event::Key::Down => libbrz::Key::Down,
        termion::event::Key::Home => libbrz::Key::Home,
        termion::event::Key::F(u) => libbrz::Key::F(u),
        termion::event::Key::Char(c) => libbrz::Key::Char(c),
        termion::event::Key::Alt(c) => libbrz::Key::Alt(c),
        termion::event::Key::Ctrl(c) => libbrz::Key::Ctrl(c),
        termion::event::Key::Null => libbrz::Key::Null,
        termion::event::Key::Esc => libbrz::Key::Esc,
        _ => unimplemented!(),
    }
}

const STYLE_RESET: u32 = 0xffff;

/// Keep track of color codes in output
///
/// This is to save on unnecessary output to terminal
/// which can generated flickering etc.
#[derive(Default)]
struct CachingAnsciWriter {
    buf: Vec<u8>,
    cur_fg: Option<u8>,
    cur_bg: Option<u8>,
    cur_bold: bool,
}

impl CachingAnsciWriter {
    fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    fn reset_fg(&mut self) -> io::Result<()> {
        if self.cur_fg.is_some() {
            self.cur_fg = None;
            write!(&mut self.buf, "{}", color::Fg(color::Reset),)?;
        }
        Ok(())
    }

    fn reset_bg(&mut self) -> io::Result<()> {
        if self.cur_bg.is_some() {
            self.cur_bg = None;
            write!(&mut self.buf, "{}", color::Bg(color::Reset),)?;
        }
        Ok(())
    }

    fn reset_style(&mut self) -> io::Result<()> {
        if self.cur_bold {
            self.cur_bold = false;
            write!(&mut self.buf, "{}", style::Reset)?;
        }
        Ok(())
    }

    fn reset_all(&mut self) -> io::Result<()> {
        self.reset_fg()?;
        self.reset_bg()?;
        self.reset_style()?;

        Ok(())
    }

    fn change_fg(&mut self, fg: color::AnsiValue) -> io::Result<()> {
        if self.cur_fg != Some(fg.0) {
            self.cur_fg = Some(fg.0);
            write!(&mut self.buf, "{}", color::Fg(fg),)?;
        }
        Ok(())
    }

    fn change_bg(&mut self, bg: color::AnsiValue) -> io::Result<()> {
        if self.cur_bg != Some(bg.0) {
            self.cur_bg = Some(bg.0);
            write!(&mut self.buf, "{}", color::Bg(bg),)?;
        }
        Ok(())
    }

    fn change_style(&mut self, bold: bool) -> io::Result<()> {
        if self.cur_bold != bold {
            self.cur_bold = bold;
            if bold {
                write!(&mut self.buf, "{}", style::Bold)?;
            } else {
                write!(&mut self.buf, "{}", style::Reset)?;
            }
        }
        Ok(())
    }

    fn set_style(&mut self, style: Option<render::Style>) -> io::Result<()> {
        if let Some(style) = style {
            if style.fg == STYLE_RESET {
                self.reset_fg()?;
            } else {
                self.change_fg(color::AnsiValue(style.fg as u8))?;
            }
            if style.bg == STYLE_RESET {
                self.reset_bg()?;
            } else {
                self.change_bg(color::AnsiValue(style.bg as u8))?;
            }

            if style.style == STYLE_RESET {
                self.reset_style()?;
            } else {
                self.change_style((style.style & 1) == 1)?;
            }
        } else {
            self.reset_all()?;
        };

        Ok(())
    }
}

impl io::Write for CachingAnsciWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Char {
    ch: char,
    style: Option<render::Style>,
}

impl Default for Char {
    fn default() -> Self {
        Char {
            ch: ' ',
            style: None,
        }
    }
}

struct Render {
    screen: AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    display_cols: usize,
    display_rows: usize,

    color_map: render::ColorMap,

    cur_buffer: Vec<Char>,
    cur_cursor_pos: Option<render::Coord>,
    prev_buffer: Vec<Char>,
}

impl Render {
    fn new() -> Result<Self> {
        let screen = AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());
        let color_map = render::ColorMap {
            default_fg: STYLE_RESET,
            default_bg: STYLE_RESET,
        };

        let mut s = Render {
            display_cols: 0,
            screen,
            display_rows: 0,
            color_map,

            cur_buffer: Vec::new(),
            cur_cursor_pos: None,
            prev_buffer: Vec::new(),
        };
        s.fix_size()?;
        Ok(s)
    }

    fn coord_to_i(&self, coord: render::Coord) -> usize {
        coord.x + coord.y * self.display_cols
    }

    fn char_at_mut(&mut self, coord: render::Coord) -> Option<&mut Char> {
        let i = self.coord_to_i(coord);
        self.cur_buffer.get_mut(i)
    }

    fn fix_size(&mut self) -> Result<()> {
        let (cols, rows) = termion::terminal_size()?;
        let cols = cols as usize;
        let rows = rows as usize;
        if self.display_cols != cols || self.display_rows != rows {
            self.display_cols = cols;
            self.display_rows = rows;
            self.cur_buffer.resize_with(cols * rows, Default::default);
            self.prev_buffer.truncate(0);
            self.cur_cursor_pos = None;
        }
        Ok(())
    }

    fn draw(&mut self, state: &State) -> Result<()> {
        state.render(self);
        let mut buf = CachingAnsciWriter::default();
        if self.prev_buffer.is_empty() {
            self.prev_buffer
                .resize_with(self.cur_buffer.len(), Char::default);
            write!(&mut buf, "{}{}", style::Reset, termion::clear::All).unwrap();
            buf.reset_all()?;
        }
        self.draw_diff(&mut buf);
        self.draw_cursor(&mut buf);

        let buf = buf.into_vec();
        dbg!(buf.len());

        self.screen.write_all(&buf)?;
        self.screen.flush()?;
        std::mem::swap(&mut self.prev_buffer, &mut self.cur_buffer);
        self.cur_buffer
            .iter_mut()
            .map(|ch| *ch = Char::default())
            .count();

        self.fix_size()?;

        Ok(())
    }

    fn draw_cursor(&mut self, buf: &mut CachingAnsciWriter) {
        if let Some(coord) = self.cur_cursor_pos {
            write!(
                buf,
                "\x1b[6 q{}{}",
                termion::cursor::Goto(coord.x as u16 + 1, coord.y as u16 + 1),
                termion::cursor::Show,
            )
            .unwrap();
        } else {
            write!(buf, "{}", termion::cursor::Hide).unwrap();
        }
    }

    fn draw_diff(&mut self, buf: &mut CachingAnsciWriter) {
        let mut needs_goto = true;

        for (ch_i, new_ch) in self.cur_buffer.iter().copied().enumerate() {
            let old_ch = self.prev_buffer[ch_i];

            if new_ch == old_ch {
                needs_goto = true;
                continue;
            }

            if needs_goto {
                needs_goto = false;

                let term_line = ch_i / self.display_cols + 1;
                let term_column = ch_i % self.display_cols + 1;

                write!(
                    buf,
                    "{}",
                    termion::cursor::Goto(term_column as u16, term_line as u16)
                )
                .unwrap();
            }

            buf.set_style(new_ch.style).unwrap();

            write!(buf, "{}", new_ch.ch).unwrap();
        }
    }
}

impl render::Renderer for Render {
    fn color_map(&self) -> &render::ColorMap {
        &self.color_map
    }

    fn dimensions(&self) -> render::Coord {
        render::Coord {
            x: self.display_cols,
            y: self.display_rows,
        }
    }

    fn put(&mut self, coord: render::Coord, ch: char, style: render::Style) {
        self.char_at_mut(coord).map(|c| {
            *c = Char {
                ch,
                style: Some(style),
            }
        });
    }

    fn set_cursor(&mut self, coord: Option<render::Coord>) {
        self.cur_cursor_pos = coord;
    }
}

/// The editor instance
///
/// Screen drawing + state handling
struct Breeze {
    state: State,
    render: Render,
}

impl Breeze {
    fn init() -> Result<Self> {
        let mut state: State = default();

        state.register_read_handler(|path| {
            Rope::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))
        });

        state.register_write_handler(|path, rope| {
            let tmp_path = path.with_extension("brz.tmp");
            rope.write_to(std::io::BufWriter::new(std::fs::File::create(&tmp_path)?))?;
            std::fs::rename(tmp_path, path)?;
            Ok(())
        });

        state.register_find_handler(|pattern| {
            Ok(ignore::Walk::new(".")
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let entry_str = entry
                        .path()
                        .to_owned()
                        .into_os_string()
                        .to_string_lossy()
                        .to_string();
                    let mut entry_str = &entry_str[..];

                    for ch in pattern.chars() {
                        if let Some(i) = entry_str.find(ch) {
                            eprintln!("{} {} {}", entry_str, ch, i);
                            entry_str = &entry_str[i + 1..];
                        } else {
                            return false;
                        }
                    }
                    return true;
                })
                .map(|entry| entry.into_path())
                .take(10)
                .collect())
        });
        let breeze = Breeze {
            state,
            render: Render::new()?,
        };

        Ok(breeze)
    }

    fn open(&mut self, path: &Path) -> Result<()> {
        self.state.open_buffer(path.to_owned());

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.draw_buffer()?;

        let stdin = std::io::stdin();
        for e in stdin.events() {
            // TODO: https://gitlab.redox-os.org/redox-os/termion/issues/151
            match e {
                Ok(Event::Key(key)) => {
                    self.state.handle_key(termion_to_brz_key(key));
                }
                Ok(Event::Unsupported(_u)) => {}
                Ok(Event::Mouse(_)) => {
                    // no animal support yet
                }
                Err(e) => panic!("{}", e),
            }

            if self.state.is_finished() {
                return Ok(());
            }
            self.draw_buffer()?;
        }
        Ok(())
    }

    fn draw_buffer(&mut self) -> Result<()> {
        self.render.draw(&self.state)?;
        /*
        let buf = self.draw_to_buf();
        self.screen.write_all(&buf)?;
        self.screen.flush()?;
        */
        Ok(())
    }

    /*
    fn draw_to_buf(&mut self) -> Vec<u8> {
        let mut buf = CachingAnsciWriter::default();

        buf.reset_color().unwrap();

        write!(&mut buf, "{}{}", style::Reset, termion::clear::All).unwrap();
        let window_height = self.display_rows - 1;
        let cursor_pos = self.state.cur_buffer().cursor_pos();
        let max_start_line = cursor_pos.line.saturating_sub(self.window_margin);
        let min_start_line = cursor_pos
            .line
            .saturating_add(self.window_margin)
            .saturating_sub(window_height);
        debug_assert!(min_start_line <= max_start_line);

        if max_start_line < self.prev_start_line {
            self.prev_start_line = max_start_line;
        }
        if self.prev_start_line < min_start_line {
            self.prev_start_line = min_start_line;
        }

        let start_line = min(
            self.prev_start_line,
            self.state
                .cur_buffer()
                .text
                .len_lines()
                .saturating_sub(window_height),
        );
        let end_line = start_line + window_height;

        let mut ch_idx = Coord {
            line: start_line,
            column: 0,
        }
        .to_idx(&self.state.cur_buffer().text)
        .0;

        for (visual_line_i, line_i) in (start_line..end_line).enumerate() {
            if line_i >= self.state.cur_buffer().text.len_lines() {
                break;
            }

            let line = self.state.cur_buffer().text.line(line_i);

            write!(
                &mut buf,
                "{}",
                termion::cursor::Goto(1, visual_line_i as u16 + 1)
            )
            .unwrap();

            let mut visual_column = 0;

            for (char_i, ch) in line.chars().enumerate().take(self.display_cols) {
                let visual_selection = self
                    .state
                    .cur_buffer()
                    .idx_selection_type(Idx(ch_idx + char_i));
                let (ch, n) = match ch {
                    '\n' => {
                        if visual_selection != VisualSelection::None {
                            (Some('↩'), 1) // alternatives: ⤶  🡿
                        } else {
                            (None, 0)
                        }
                    }
                    '\t' => (Some('.'), distance_to_next_tabstop(visual_column, 4)),
                    ch => (Some(ch), 1),
                };

                if let Some(ch) = ch {
                    match visual_selection {
                        VisualSelection::DirectionMarker => {
                            buf.change_color(color::AnsiValue(14), color::AnsiValue(10), false)
                                .unwrap();
                        }
                        VisualSelection::Selection => {
                            buf.change_color(color::AnsiValue(16), color::AnsiValue(4), false)
                                .unwrap();
                        }
                        VisualSelection::None => {
                            buf.reset_color().unwrap();
                        }
                    }
                    for _ in 0..n {
                        write!(&mut buf, "{}", ch).unwrap();
                    }
                }
                visual_column += n;
            }
            ch_idx += line.len_chars();
        }

        for (i, (key, action)) in self
            .state
            .get_mode()
            .available_actions()
            .iter()
            .enumerate()
            .take(10)
        {
            write!(
                &mut buf,
                "{}{} {}",
                termion::cursor::Goto(
                    (self.display_cols - self.display_cols / 4) as u16,
                    self.display_rows.saturating_sub(12).saturating_add(i) as u16
                ),
                key,
                action.help()
            )
            .unwrap();
        }

        // status
        buf.reset_color().unwrap();
        if let Some(cmd_str) = self.state.cmd_string() {
            write!(
                &mut buf,
                "{}{}",
                termion::cursor::Goto(1, self.display_rows as u16),
                cmd_str,
            )
            .unwrap();
        }
        let right_side_status = format!(
            "{}",
            self.state.mode_name(),
            // self.state.mode_num_prefix_str(),
        );
        write!(
            &mut buf,
            "{}{}",
            termion::cursor::Goto(
                (self.display_cols - right_side_status.len()) as u16,
                self.display_rows as u16
            ),
            right_side_status
        )
        .unwrap();

        let cursor_visual = self.state.cur_buffer().to_visual(cursor_pos);
        // cursor
        write!(
            &mut buf,
            "\x1b[6 q{}{}",
            termion::cursor::Goto(
                cursor_visual.column as u16 + 1,
                (cursor_visual.line - start_line) as u16 + 1
            ),
            termion::cursor::Show,
        )
        .unwrap();
        buf.into_vec()
    }
    */
}

fn run() -> Result<()> {
    let opt = opts::Opts::from_args();
    let mut brz = Breeze::init()?;

    for path in opt.inputs {
        brz.open(&path)?;
    }

    brz.run()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        handle_error(&e)
    }
}

fn handle_error(error: &failure::Error) {
    eprintln!("error: {}", error);

    for e in error.iter_chain() {
        eprintln!("caused by: {}", e);
    }

    eprintln!("backtrace: {:?}", error.backtrace());

    process::exit(1);
}
