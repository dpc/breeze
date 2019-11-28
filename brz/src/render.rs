use termion::color;
use termion::raw::IntoRawMode;
use termion::screen::*;
use termion::style;

use libbrz::render;
use libbrz::{prelude::*, state::State};
use std::io::{self, Write};

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

    fn set_style(&mut self, style: render::Style) -> io::Result<()> {
        if let Some(fg) = style.fg {
            self.change_fg(color::AnsiValue(fg as u8))?;
        } else {
            self.reset_fg()?;
        }
        if let Some(bg) = style.bg {
            self.change_bg(color::AnsiValue(bg as u8))?;
        } else {
            self.reset_bg()?;
        }

        if let Some(style) = style.style {
            self.change_style((style & 1) == 1)?;
        } else {
            self.reset_style()?;
        }

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
    style: render::Style,
}

impl Default for Char {
    fn default() -> Self {
        Char {
            ch: ' ',
            style: render::Style::default(),
        }
    }
}

pub struct Render {
    screen: AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    display_cols: usize,
    display_rows: usize,

    color_map: render::ColorMap,

    cur_buffer: Vec<Char>,
    cur_cursor_pos: Option<render::Coord>,
    prev_buffer: Vec<Char>,
}

impl Render {
    pub fn new() -> Result<Self> {
        let screen = AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());
        let color_map = render::ColorMap {
            default: render::Style::default(),
            actions: render::Style {
                fg: Some(8),
                bg: Some(14),
                ..Default::default()
            },
            line_num: render::Style {
                fg: Some(10),
                ..Default::default()
            },
            direction_marker: render::Style {
                fg: Some(16),
                bg: Some(4),
                ..Default::default()
            },
            selection: render::Style {
                fg: Some(16),
                bg: Some(4),
                ..Default::default()
            },
            special: render::Style {
                fg: Some(14),
                ..Default::default()
            },
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

    pub fn draw(&mut self, state: &State) -> Result<()> {
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
        buf.reset_all()?;

        let buf = buf.into_vec();
        buf.len();

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
        self.char_at_mut(coord).map(|c| *c = Char { ch, style });
    }

    fn set_cursor(&mut self, coord: Option<render::Coord>) {
        self.cur_cursor_pos = coord;
    }
}
