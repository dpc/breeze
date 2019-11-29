#![allow(dead_code)]

pub mod action;
pub mod buffer;
pub mod idx;
pub mod mode;
pub mod position;
pub mod range;
pub mod selection;

pub mod prelude;
pub mod render;
pub mod state;
pub mod util;

pub use self::idx::Idx;
pub use self::mode::Mode;
pub use self::position::Position;
pub use self::state::State;
use std::cmp;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NaturalyOrderedKey(pub Key);

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Key::*;
        match *self {
            F(c) => f.pad(&format!("f{}", c)),
            Char(c) => f.pad(&format!("{}", c)),
            Alt(c) => f.pad(&format!("a-{}", c)),
            Ctrl(c) => f.pad(&format!("c-{}", c)),
            Esc => f.pad("esc"),
            _ => f.pad("?"),
        }
    }
}

impl NaturalyOrderedKey {
    fn ordering_keys(self) -> (usize, char, usize) {
        use self::Key::*;
        match self.0 {
            F(c) => (9, ('0' as u8 + c) as char, 0),
            Char(c) => (
                1,
                c.to_ascii_lowercase(),
                1 + c.is_ascii_uppercase() as usize,
            ),
            Ctrl(c) => (
                1,
                c.to_ascii_lowercase(),
                3 + c.is_ascii_uppercase() as usize,
            ),
            Alt(c) => (
                1,
                c.to_ascii_lowercase(),
                5 + c.is_ascii_uppercase() as usize,
            ),
            Esc => (0, '\0', 0),
            _ => (10, '?', 0),
        }
    }
}

impl cmp::Ord for NaturalyOrderedKey {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let (s_group, s_key, s_mod) = self.ordering_keys();
        let (o_group, o_key, o_mod) = other.ordering_keys();
        s_group
            .cmp(&o_group)
            .then(s_key.cmp(&o_key))
            .then(s_mod.cmp(&o_mod))
            .then(self.0.cmp(&other.0))
    }
}

impl cmp::PartialOrd for NaturalyOrderedKey {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
