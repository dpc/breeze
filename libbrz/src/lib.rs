#![allow(dead_code)]

pub mod buffer;
pub mod coord;
pub mod idx;
pub mod mode;
pub mod range;
pub mod selection;

pub mod prelude;
pub mod state;
pub mod util;

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
