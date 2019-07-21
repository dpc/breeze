use std::path::Path;

use structopt::StructOpt;
use termion::event::Event;
use termion::input::TermRead;

use ropey::Rope;
use std;
use std::process;

mod opts;
mod render;

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

/// The editor instance
///
/// Screen drawing + state handling
struct Breeze {
    state: State,
    render: render::Render,
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
                .filter(|entry| entry.file_type().map(|f| f.is_file()).unwrap_or(false))
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
                            entry_str = &entry_str[i + 1..];
                        } else {
                            return false;
                        }
                    }
                    true
                })
                .map(|entry| entry.into_path())
                .take(10)
                .collect())
        });
        let breeze = Breeze {
            state,
            render: render::Render::new()?,
        };

        Ok(breeze)
    }

    fn open(&mut self, path: &Path) -> Result<()> {
        self.state.open_buffer(path);

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
        Ok(())
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
