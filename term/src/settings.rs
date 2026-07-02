// Adapted from iced_term's settings.rs (MIT, see ../LICENSE-iced_term):
// `BackendSettings` no longer describes a local shell to spawn — it carries
// the channels that connect the widget to whatever transport drives it.
use crate::backend::TermCommand;
use crate::ColorPalette;
use iced::Font;
use tokio::sync::mpsc;

pub struct Settings {
    pub font: FontSettings,
    pub theme: ThemeSettings,
    pub backend: BackendSettings,
}

pub struct BackendSettings {
    /// Outgoing: keystrokes and resize requests produced by the widget.
    pub input: mpsc::Sender<TermCommand>,
    /// Incoming: raw bytes received from the transport (e.g. SSH channel data).
    pub output: mpsc::Receiver<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub size: f32,
    pub scale_factor: f32,
    pub font_type: Font,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self { size: 14.0, scale_factor: 1.3, font_type: Font::MONOSPACE }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ThemeSettings {
    pub color_pallete: Box<ColorPalette>,
}

impl ThemeSettings {
    pub fn new(color_pallete: Box<ColorPalette>) -> Self {
        Self { color_pallete }
    }
}
