use std::path::Path;

use std::cmp::min;
use std::io::{self, Write};
use structopt::StructOpt;
use termion::color;
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;
use termion::style;

use ropey::Rope;
use std;
use std::process;

mod opts;

use libbrz::{buffer::*, coord::*, idx::Idx, prelude::*, state::State};

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

    fn reset_color(&mut self) -> io::Result<()> {
        if self.cur_fg.is_some() {
            self.cur_fg = None;
            write!(&mut self.buf, "{}", color::Fg(color::Reset),)?;
        }

        if self.cur_bg.is_some() {
            self.cur_bg = None;
            write!(&mut self.buf, "{}", color::Bg(color::Reset),)?;
        }
        if self.cur_bold {
            self.cur_bold = false;
            eprintln!("sdf");
            write!(&mut self.buf, "{}", style::Reset)?;
        }
        Ok(())
    }

    fn change_color(
        &mut self,
        fg: color::AnsiValue,
        bg: color::AnsiValue,
        bold: bool,
    ) -> io::Result<()> {
        if self.cur_fg != Some(fg.0) {
            self.cur_fg = Some(fg.0);
            write!(&mut self.buf, "{}", color::Fg(fg),)?;
        }

        if self.cur_bg != Some(bg.0) {
            self.cur_bg = Some(bg.0);
            write!(&mut self.buf, "{}", color::Bg(bg),)?;
        }

        if self.cur_bold != bold {
            self.cur_bold = bold;
            if bold {
                write!(&mut self.buf, "{}", style::Bold)?;
            } else {
                eprintln!("df");
                write!(&mut self.buf, "{}", style::Reset)?;
            }
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
        let mut breeze = Breeze {
            state,
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
        self.state.open_buffer(path.to_owned());

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.draw_buffer()?;

        let stdin = std::io::stdin();
        for e in stdin.events() {
            match e {
                Ok(Event::Key(key)) => {
                    eprintln!("{:?}", key);
                    self.state.handle_key(termion_to_brz_key(key));
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

            if self.state.is_finished() {
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
            "{}",
            termion::cursor::Goto(
                (self.display_cols - right_side_status.len()) as u16,
                self.display_rows as u16
            ),
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
