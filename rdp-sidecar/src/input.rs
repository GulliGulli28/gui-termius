//! Maps browser input (DOM `KeyboardEvent.code`/`MouseEvent.button`, both
//! forwarded as-is over `rdp-ipc`) onto the RDP wire's PS/2 Set 1 scancodes
//! and `ironrdp_input::Operation`s. `ironrdp_input::Database` (see
//! `rdp-sidecar/Cargo.toml`'s `input` feature) already owns the
//! press/release state machine and `FastPathInputEvent` encoding — this
//! module only supplies the piece it doesn't: turning a `KeyboardEvent.code`
//! string into a scancode.

use ironrdp::input::Scancode;

/// `code` is a DOM `KeyboardEvent.code` value (physical key, layout-independent
/// — see <https://www.w3.org/TR/uievents-code/>). Covers the keys needed for
/// normal desktop use; anything unmapped is silently ignored rather than
/// guessed at, since a wrong scancode is worse than a dropped keystroke.
pub fn scancode_for(code: &str) -> Option<Scancode> {
    let (extended, raw) = match code {
        "Escape" => (false, 0x01),
        "Digit1" => (false, 0x02),
        "Digit2" => (false, 0x03),
        "Digit3" => (false, 0x04),
        "Digit4" => (false, 0x05),
        "Digit5" => (false, 0x06),
        "Digit6" => (false, 0x07),
        "Digit7" => (false, 0x08),
        "Digit8" => (false, 0x09),
        "Digit9" => (false, 0x0A),
        "Digit0" => (false, 0x0B),
        "Minus" => (false, 0x0C),
        "Equal" => (false, 0x0D),
        "Backspace" => (false, 0x0E),
        "Tab" => (false, 0x0F),
        "KeyQ" => (false, 0x10),
        "KeyW" => (false, 0x11),
        "KeyE" => (false, 0x12),
        "KeyR" => (false, 0x13),
        "KeyT" => (false, 0x14),
        "KeyY" => (false, 0x15),
        "KeyU" => (false, 0x16),
        "KeyI" => (false, 0x17),
        "KeyO" => (false, 0x18),
        "KeyP" => (false, 0x19),
        "BracketLeft" => (false, 0x1A),
        "BracketRight" => (false, 0x1B),
        "Enter" => (false, 0x1C),
        "ControlLeft" => (false, 0x1D),
        "KeyA" => (false, 0x1E),
        "KeyS" => (false, 0x1F),
        "KeyD" => (false, 0x20),
        "KeyF" => (false, 0x21),
        "KeyG" => (false, 0x22),
        "KeyH" => (false, 0x23),
        "KeyJ" => (false, 0x24),
        "KeyK" => (false, 0x25),
        "KeyL" => (false, 0x26),
        "Semicolon" => (false, 0x27),
        "Quote" => (false, 0x28),
        "Backquote" => (false, 0x29),
        "ShiftLeft" => (false, 0x2A),
        "Backslash" => (false, 0x2B),
        "KeyZ" => (false, 0x2C),
        "KeyX" => (false, 0x2D),
        "KeyC" => (false, 0x2E),
        "KeyV" => (false, 0x2F),
        "KeyB" => (false, 0x30),
        "KeyN" => (false, 0x31),
        "KeyM" => (false, 0x32),
        "Comma" => (false, 0x33),
        "Period" => (false, 0x34),
        "Slash" => (false, 0x35),
        "ShiftRight" => (false, 0x36),
        "NumpadMultiply" => (false, 0x37),
        "AltLeft" => (false, 0x38),
        "Space" => (false, 0x39),
        "CapsLock" => (false, 0x3A),
        "F1" => (false, 0x3B),
        "F2" => (false, 0x3C),
        "F3" => (false, 0x3D),
        "F4" => (false, 0x3E),
        "F5" => (false, 0x3F),
        "F6" => (false, 0x40),
        "F7" => (false, 0x41),
        "F8" => (false, 0x42),
        "F9" => (false, 0x43),
        "F10" => (false, 0x44),
        "NumLock" => (false, 0x45),
        "ScrollLock" => (false, 0x46),
        "Numpad7" => (false, 0x47),
        "Numpad8" => (false, 0x48),
        "Numpad9" => (false, 0x49),
        "NumpadSubtract" => (false, 0x4A),
        "Numpad4" => (false, 0x4B),
        "Numpad5" => (false, 0x4C),
        "Numpad6" => (false, 0x4D),
        "NumpadAdd" => (false, 0x4E),
        "Numpad1" => (false, 0x4F),
        "Numpad2" => (false, 0x50),
        "Numpad3" => (false, 0x51),
        "Numpad0" => (false, 0x52),
        "NumpadDecimal" => (false, 0x53),
        "IntlBackslash" => (false, 0x56),
        "F11" => (false, 0x57),
        "F12" => (false, 0x58),

        "NumpadEnter" => (true, 0x1C),
        "ControlRight" => (true, 0x1D),
        "NumpadDivide" => (true, 0x35),
        "AltRight" => (true, 0x38),
        "Home" => (true, 0x47),
        "ArrowUp" => (true, 0x48),
        "PageUp" => (true, 0x49),
        "ArrowLeft" => (true, 0x4B),
        "ArrowRight" => (true, 0x4D),
        "End" => (true, 0x4F),
        "ArrowDown" => (true, 0x50),
        "PageDown" => (true, 0x51),
        "Insert" => (true, 0x52),
        "Delete" => (true, 0x53),
        "MetaLeft" => (true, 0x5B),
        "MetaRight" => (true, 0x5C),
        "ContextMenu" => (true, 0x5D),

        _ => return None,
    };
    Some(Scancode::from_u8(extended, raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_keys() {
        assert_eq!(scancode_for("KeyA"), Some(Scancode::from_u8(false, 0x1E)));
        assert_eq!(scancode_for("Enter"), Some(Scancode::from_u8(false, 0x1C)));
        assert_eq!(scancode_for("ArrowLeft"), Some(Scancode::from_u8(true, 0x4B)));
        assert_eq!(scancode_for("ControlRight"), Some(Scancode::from_u8(true, 0x1D)));
    }

    #[test]
    fn unknown_code_maps_to_none() {
        assert_eq!(scancode_for("SomeFutureKey"), None);
    }
}
