//! A terminal emulator widget for iced, adapted from `iced_term`
//! (MIT, see LICENSE-iced_term) to be driven by an arbitrary byte transport
//! (SSH, local PTY, telnet, ...) instead of always spawning a local shell.

pub mod actions;
pub mod bindings;
pub mod settings;

mod backend;
mod font;
mod terminal;
mod theme;
mod view;

pub use alacritty_terminal::event::Event as AlacrittyEvent;
pub use alacritty_terminal::index::Point as AlacrittyPoint;
pub use alacritty_terminal::selection::SelectionType;
pub use alacritty_terminal::term::TermMode;
pub use backend::Command as BackendCommand;
pub use backend::{LinkAction, MouseButton, TermCommand};
pub use terminal::{Command, Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
