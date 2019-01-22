#![allow(dead_code)]

mod prelude;

use self::prelude::*;

use std::path::Path;

use std::cmp::min;
use std::io::{self, Write};
use structopt::StructOpt;
use termion::color;
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;

use ropey::Rope;

mod buffer;
mod coord;
mod idx;
mod mode;
mod opts;
mod selection;

use crate::{buffer::*, coord::*, idx::Idx, mode::*};

/// Keep track of color codes in output
///
/// This is to save on unnecessary output to terminal
/// which can generated flickering etc.
#[derive(Default)]
struct CachingAnsciWriter {
    buf: Vec<u8>,
    cur_fg: Option<u8>,
    cur_bg: Option<u8>,
}

impl CachingAnsciWriter {
    fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    fn reset_color(&mut self) -> io::Result<()> {
        if self.cur_fg.is_some() {
            self.cur_fg = None;
            write!(&mut self.buf, "{}", color::Fg(color::Reset),)?;
        }

        if self.cur_bg.is_some() {
            self.cur_bg = None;
            write!(&mut self.buf, "{}", color::Bg(color::Reset),)?;
        }
        Ok(())
    }

    fn change_color(&mut self, fg: color::AnsiValue, bg: color::AnsiValue) -> io::Result<()> {
        if self.cur_fg != Some(fg.0) {
            self.cur_fg = Some(fg.0);
            write!(&mut self.buf, "{}", color::Fg(fg),)?;
        }

        if self.cur_bg != Some(bg.0) {
            self.cur_bg = Some(bg.0);
            write!(&mut self.buf, "{}", color::Bg(bg),)?;
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

/// The editor state
#[derive(Clone)]
pub struct State {
    quit: bool,
    mode: Mode,
    buffer: Buffer,
    yanked: Vec<Rope>,
}

impl Default for State {
    fn default() -> Self {
        State {
            quit: false,
            mode: Mode::default(),
            buffer: default(),
            yanked: vec![],
        }
    }
}

/// The editor instance
///
/// Screen drawing + state handling
struct Breeze {
    state: State,
    screen: AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    display_cols: usize,
    display_rows: usize,
    prev_start_line: usize,
    window_margin: usize,
}

impl Breeze {
    fn init() -> Result<Self> {
        let screen = AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());

        let mut breeze = Breeze {
            state: default(),
            display_cols: 0,
            screen,
            display_rows: 0,
            prev_start_line: 0,
            window_margin: 0,
        };
        breeze.fix_size()?;

        Ok(breeze)
    }

    fn fix_size(&mut self) -> Result<()> {
        let (cols, rows) = termion::terminal_size()?;
        self.display_cols = cols as usize;
        self.display_rows = rows as usize;
        self.window_margin = self.display_rows / 5;
        Ok(())
    }

    fn open(&mut self, path: &Path) -> Result<()> {
        let text = Rope::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?;
        self.state.buffer = Buffer::from_text(text);
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.draw_buffer()?;

        let stdin = std::io::stdin();
        for e in stdin.events() {
            match e {
                Ok(Event::Key(key)) => {
                    self.state = self.state.mode.handle(self.state.clone(), key);
                }
                Ok(Event::Unsupported(_u)) => {
                    eprintln!("{:?}", _u);
                    self.fix_size()?;
                }
                Ok(Event::Mouse(_)) => {
                    // no animal support yet
                }
                Err(e) => panic!("{}", e),
            }

            if self.state.quit {
                return Ok(());
            }
            self.draw_buffer()?;
        }
        Ok(())
    }

    fn draw_buffer(&mut self) -> Result<()> {
        let buf = self.draw_to_buf();
        self.screen.write_all(&buf)?;
        self.screen.flush()?;
        Ok(())
    }

    fn draw_to_buf(&mut self) -> Vec<u8> {
        let mut buf = CachingAnsciWriter::default();

        buf.reset_color().unwrap();

        write!(&mut buf, "{}", termion::clear::All).unwrap();
        let window_height = self.display_rows - 1;
        let cursor_pos = self.state.buffer.cursor_pos();
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
            self.state.buffer.text.len_lines() - window_height,
        );
        let end_line = start_line + window_height;

        let mut ch_idx = Coord {
            line: start_line,
            column: 0,
        }
        .to_idx(&self.state.buffer.text)
        .0;

        for (visual_line_i, line_i) in (start_line..end_line).enumerate() {
            if line_i >= self.state.buffer.text.len_lines() {
                break;
            }

            let line = self.state.buffer.text.line(line_i);

            write!(
                &mut buf,
                "{}",
                termion::cursor::Goto(1, visual_line_i as u16 + 1)
            )
            .unwrap();
            for (char_i, ch) in line.chars().enumerate().take(self.display_cols) {
                let visual_selection = self.state.buffer.idx_selection_type(Idx(ch_idx + char_i));
                let ch = if ch == '\n' {
                    if visual_selection != VisualSelection::None {
                        Some('·')
                    } else {
                        None
                    }
                } else {
                    Some(ch)
                };

                if let Some(ch) = ch {
                    match visual_selection {
                        VisualSelection::DirectionMarker => {
                            buf.change_color(color::AnsiValue(14), color::AnsiValue(10))
                                .unwrap();
                        }
                        VisualSelection::Selection => {
                            buf.change_color(color::AnsiValue(16), color::AnsiValue(4))
                                .unwrap();
                        }
                        VisualSelection::None => {
                            buf.reset_color().unwrap();
                        }
                    }
                    write!(&mut buf, "{}", ch).unwrap();
                }
            }
            ch_idx += line.len_chars();
        }

        // status
        buf.reset_color().unwrap();
        write!(
            &mut buf,
            "{}{} {}",
            termion::cursor::Goto(1, self.display_rows as u16),
            self.state.mode.name(),
            self.state.mode.num_prefix_str(),
        )
        .unwrap();

        // cursor
        write!(
            &mut buf,
            "\x1b[6 q{}{}",
            termion::cursor::Goto(
                cursor_pos.column as u16 + 1,
                (cursor_pos.line - start_line) as u16 + 1
            ),
            termion::cursor::Show,
        )
        .unwrap();
        buf.into_vec()
    }
}

fn main() -> Result<()> {
    let opt = opts::Opts::from_args();
    let mut brz = Breeze::init()?;

    if let Some(path) = opt.input {
        brz.open(&path)?;
    }

    brz.run()?;
    Ok(())
}
