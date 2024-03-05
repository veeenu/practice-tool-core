use std::str::FromStr;

use imgui::Ui;
use serde::Deserialize;

const REPR_MAP: &[(imgui::Key, &str)] = &[
    (imgui::Key::Tab, "tab"),
    (imgui::Key::LeftArrow, "left"),
    (imgui::Key::RightArrow, "righ"),
    (imgui::Key::UpArrow, "up"),
    (imgui::Key::DownArrow, "down"),
    (imgui::Key::PageUp, "pgup"),
    (imgui::Key::PageDown, "pgdown"),
    (imgui::Key::Home, "home"),
    (imgui::Key::End, "end"),
    (imgui::Key::Insert, "insert"),
    (imgui::Key::Delete, "delete"),
    (imgui::Key::Backspace, "backspace"),
    (imgui::Key::Space, "space"),
    (imgui::Key::Enter, "enter"),
    (imgui::Key::Escape, "escape"),
    (imgui::Key::LeftCtrl, "lctrl"),
    (imgui::Key::LeftShift, "lshift"),
    (imgui::Key::LeftAlt, "lalt"),
    (imgui::Key::LeftSuper, "lsuper"),
    (imgui::Key::RightCtrl, "rctrl"),
    (imgui::Key::RightShift, "rshift"),
    (imgui::Key::RightAlt, "ralt"),
    (imgui::Key::RightSuper, "rsuper"),
    (imgui::Key::Menu, "menu"),
    (imgui::Key::Alpha0, "0"),
    (imgui::Key::Alpha1, "1"),
    (imgui::Key::Alpha2, "2"),
    (imgui::Key::Alpha3, "3"),
    (imgui::Key::Alpha4, "4"),
    (imgui::Key::Alpha5, "5"),
    (imgui::Key::Alpha6, "6"),
    (imgui::Key::Alpha7, "7"),
    (imgui::Key::Alpha8, "8"),
    (imgui::Key::Alpha9, "9"),
    (imgui::Key::A, "a"),
    (imgui::Key::B, "b"),
    (imgui::Key::C, "c"),
    (imgui::Key::D, "d"),
    (imgui::Key::E, "e"),
    (imgui::Key::F, "f"),
    (imgui::Key::G, "g"),
    (imgui::Key::H, "h"),
    (imgui::Key::I, "i"),
    (imgui::Key::J, "j"),
    (imgui::Key::K, "k"),
    (imgui::Key::L, "l"),
    (imgui::Key::M, "m"),
    (imgui::Key::N, "n"),
    (imgui::Key::O, "o"),
    (imgui::Key::P, "p"),
    (imgui::Key::Q, "q"),
    (imgui::Key::R, "r"),
    (imgui::Key::S, "s"),
    (imgui::Key::T, "t"),
    (imgui::Key::U, "u"),
    (imgui::Key::V, "v"),
    (imgui::Key::W, "w"),
    (imgui::Key::X, "x"),
    (imgui::Key::Y, "y"),
    (imgui::Key::Z, "z"),
    (imgui::Key::F1, "f1"),
    (imgui::Key::F2, "f2"),
    (imgui::Key::F3, "f3"),
    (imgui::Key::F4, "f4"),
    (imgui::Key::F5, "f5"),
    (imgui::Key::F6, "f6"),
    (imgui::Key::F7, "f7"),
    (imgui::Key::F8, "f8"),
    (imgui::Key::F9, "f9"),
    (imgui::Key::F10, "f10"),
    (imgui::Key::F11, "f11"),
    (imgui::Key::F12, "f12"),
    (imgui::Key::Apostrophe, "'"),
    (imgui::Key::Comma, "),"),
    (imgui::Key::Minus, "-"),
    (imgui::Key::Period, "."),
    (imgui::Key::Slash, "/"),
    (imgui::Key::Semicolon, ";"),
    (imgui::Key::Equal, "="),
    (imgui::Key::LeftBracket, "["),
    (imgui::Key::Backslash, "\\"),
    (imgui::Key::RightBracket, "]"),
    (imgui::Key::GraveAccent, "`"),
    (imgui::Key::CapsLock, "capslock"),
    (imgui::Key::ScrollLock, "scrolllock"),
    (imgui::Key::NumLock, "numlock"),
    (imgui::Key::PrintScreen, "printscreen"),
    (imgui::Key::Pause, "pause"),
    (imgui::Key::Keypad0, "kp0"),
    (imgui::Key::Keypad1, "kp1"),
    (imgui::Key::Keypad2, "kp2"),
    (imgui::Key::Keypad3, "kp3"),
    (imgui::Key::Keypad4, "kp4"),
    (imgui::Key::Keypad5, "kp5"),
    (imgui::Key::Keypad6, "kp6"),
    (imgui::Key::Keypad7, "kp7"),
    (imgui::Key::Keypad8, "kp8"),
    (imgui::Key::Keypad9, "kp9"),
    (imgui::Key::KeypadDecimal, "kpdecimal"),
    (imgui::Key::KeypadDivide, "kpdivide"),
    (imgui::Key::KeypadMultiply, "kpmultiply"),
    (imgui::Key::KeypadSubtract, "kpsubtract"),
    (imgui::Key::KeypadAdd, "kpadd"),
    (imgui::Key::KeypadEnter, "kpenter"),
    (imgui::Key::KeypadEqual, "kpequal"),
    (imgui::Key::GamepadStart, "gamepadstart"),
    (imgui::Key::GamepadBack, "gamepadback"),
    (imgui::Key::GamepadFaceLeft, "gamepadfaceleft"),
    (imgui::Key::GamepadFaceRight, "gamepadfaceright"),
    (imgui::Key::GamepadFaceUp, "gamepadfaceup"),
    (imgui::Key::GamepadFaceDown, "gamepadfacedown"),
    (imgui::Key::GamepadDpadLeft, "gamepaddpadleft"),
    (imgui::Key::GamepadDpadRight, "gamepaddpadright"),
    (imgui::Key::GamepadDpadUp, "gamepaddpadup"),
    (imgui::Key::GamepadDpadDown, "gamepaddpaddown"),
    (imgui::Key::GamepadL1, "gamepadl1"),
    (imgui::Key::GamepadR1, "gamepadr1"),
    (imgui::Key::GamepadL2, "gamepadl2"),
    (imgui::Key::GamepadR2, "gamepadr2"),
    (imgui::Key::GamepadL3, "gamepadl3"),
    (imgui::Key::GamepadR3, "gamepadr3"),
    (imgui::Key::GamepadLStickLeft, "gamepadlstickleft"),
    (imgui::Key::GamepadLStickRight, "gamepadlstickright"),
    (imgui::Key::GamepadLStickUp, "gamepadlstickup"),
    (imgui::Key::GamepadLStickDown, "gamepadlstickdown"),
    (imgui::Key::GamepadRStickLeft, "gamepadrstickleft"),
    (imgui::Key::GamepadRStickRight, "gamepadrstickright"),
    (imgui::Key::GamepadRStickUp, "gamepadrstickup"),
    (imgui::Key::GamepadRStickDown, "gamepadrstickdown"),
    (imgui::Key::MouseLeft, "mouseleft"),
    (imgui::Key::MouseRight, "mouseright"),
    (imgui::Key::MouseMiddle, "mousemiddle"),
    (imgui::Key::MouseX1, "mousex1"),
    (imgui::Key::MouseX2, "mousex2"),
    (imgui::Key::MouseWheelX, "mousewheelx"),
    (imgui::Key::MouseWheelY, "mousewheely"),
    (imgui::Key::ReservedForModCtrl, "reservedformodctrl"),
    (imgui::Key::ReservedForModShift, "reservedformodshift"),
    (imgui::Key::ReservedForModAlt, "reservedformodalt"),
    (imgui::Key::ReservedForModSuper, "reservedformodsuper"),
    (imgui::Key::ModCtrl, "ctrl"),
    (imgui::Key::ModShift, "shift"),
    (imgui::Key::ModAlt, "alt"),
    (imgui::Key::ModSuper, "super"),
    (imgui::Key::ModShortcut, "shortcut"),
];

const MOD_REPR_MAP: &[(Modifier, &str)] = &[
    (Modifier::LeftCtrl, "lctrl"),
    (Modifier::LeftShift, "lshift"),
    (Modifier::LeftAlt, "lalt"),
    (Modifier::LeftSuper, "lsuper"),
    (Modifier::RightCtrl, "rctrl"),
    (Modifier::RightShift, "rshift"),
    (Modifier::RightAlt, "ralt"),
    (Modifier::RightSuper, "rsuper"),
    (Modifier::ModCtrl, "ctrl"),
    (Modifier::ModShift, "shift"),
    (Modifier::ModAlt, "alt"),
    (Modifier::ModSuper, "super"),
];

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    LeftCtrl = imgui::sys::ImGuiKey_LeftCtrl,
    LeftShift = imgui::sys::ImGuiKey_LeftShift,
    LeftAlt = imgui::sys::ImGuiKey_LeftAlt,
    LeftSuper = imgui::sys::ImGuiKey_LeftSuper,
    RightCtrl = imgui::sys::ImGuiKey_RightCtrl,
    RightShift = imgui::sys::ImGuiKey_RightShift,
    RightAlt = imgui::sys::ImGuiKey_RightAlt,
    RightSuper = imgui::sys::ImGuiKey_RightSuper,
    ModCtrl = imgui::sys::ImGuiMod_Ctrl,
    ModShift = imgui::sys::ImGuiMod_Shift,
    ModAlt = imgui::sys::ImGuiMod_Alt,
    ModSuper = imgui::sys::ImGuiMod_Super,
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = MOD_REPR_MAP
            .iter()
            .find_map(|&(key, val)| if key == *self { Some(val) } else { None })
            .unwrap_or("???");

        write!(f, "{repr}")
    }
}

impl From<Modifier> for imgui::Key {
    fn from(val: Modifier) -> Self {
        match val {
            Modifier::LeftCtrl => imgui::Key::LeftCtrl,
            Modifier::LeftShift => imgui::Key::LeftShift,
            Modifier::LeftAlt => imgui::Key::LeftAlt,
            Modifier::LeftSuper => imgui::Key::LeftSuper,
            Modifier::RightCtrl => imgui::Key::RightCtrl,
            Modifier::RightShift => imgui::Key::RightShift,
            Modifier::RightAlt => imgui::Key::RightAlt,
            Modifier::RightSuper => imgui::Key::RightSuper,
            Modifier::ModCtrl => imgui::Key::ModCtrl,
            Modifier::ModShift => imgui::Key::ModShift,
            Modifier::ModAlt => imgui::Key::ModAlt,
            Modifier::ModSuper => imgui::Key::ModSuper,
        }
    }
}

impl FromStr for Modifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MOD_REPR_MAP
            .iter()
            .find_map(|&(key, val)| if val == s { Some(key) } else { None })
            .ok_or_else(|| format!("Could not find modifier: \"{s}\""))
    }
}

impl Modifier {
    pub fn is_down(&self, ui: &Ui) -> bool {
        match self {
            Modifier::LeftCtrl
            | Modifier::LeftShift
            | Modifier::LeftAlt
            | Modifier::LeftSuper
            | Modifier::RightCtrl
            | Modifier::RightShift
            | Modifier::RightAlt
            | Modifier::RightSuper => ui.is_key_down((*self).into()),
            Modifier::ModCtrl => ui.io().key_ctrl,
            Modifier::ModShift => ui.io().key_shift,
            Modifier::ModAlt => ui.io().key_alt,
            Modifier::ModSuper => ui.io().key_super,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModifierState {
    key_ctrl: bool,
    key_shift: bool,
    key_alt: bool,
    key_super: bool,
}

impl From<&Ui> for ModifierState {
    fn from(value: &Ui) -> Self {
        let io = value.io();
        Self {
            key_ctrl: io.key_ctrl,
            key_shift: io.key_shift,
            key_alt: io.key_alt,
            key_super: io.key_super,
        }
    }
}

impl From<[Option<Modifier>; 3]> for ModifierState {
    fn from(value: [Option<Modifier>; 3]) -> Self {
        let mut modifier_state =
            Self { key_ctrl: false, key_shift: false, key_alt: false, key_super: false };

        for modifier in value.into_iter().flatten() {
            match modifier {
                Modifier::LeftCtrl | Modifier::RightCtrl | Modifier::ModCtrl => {
                    modifier_state.key_ctrl = true
                },
                Modifier::LeftShift | Modifier::RightShift | Modifier::ModShift => {
                    modifier_state.key_shift = true
                },
                Modifier::LeftAlt | Modifier::RightAlt | Modifier::ModAlt => {
                    modifier_state.key_alt = true
                },
                Modifier::LeftSuper | Modifier::RightSuper | Modifier::ModSuper => {
                    modifier_state.key_super = true
                },
            }
        }

        modifier_state
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(try_from = "String")]
pub struct Key {
    key: imgui::Key,
    modifiers: [Option<Modifier>; 3],
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = REPR_MAP
            .iter()
            .find_map(|&(key, val)| if key == self.key { Some(val) } else { None })
            .unwrap_or("???");

        for modifier in self.modifiers.into_iter().flatten() {
            write!(f, "{modifier}+")?;
        }

        write!(f, "{repr}")
    }
}

impl TryFrom<&str> for Key {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut chunks = s.split('+').rev();
        let key_chunk =
            chunks.next().ok_or_else(|| format!("Could not parse key: \"{s}\""))?.to_lowercase();

        let key = REPR_MAP
            .iter()
            .find_map(|&(key, val)| if val == key_chunk { Some(key) } else { None })
            .ok_or_else(|| format!("Could not find key: \"{key_chunk}\""))?;

        let mut modifier_chunks = chunks.rev().map(Modifier::from_str);

        let mut modifiers = [None; 3];
        if let Some(modifier) = modifier_chunks.next() {
            modifiers[0] = Some(modifier?);
        }
        if let Some(modifier) = modifier_chunks.next() {
            modifiers[1] = Some(modifier?);
        }
        if let Some(modifier) = modifier_chunks.next() {
            modifiers[2] = Some(modifier?);
        }

        Ok(Self { key, modifiers })
    }
}

impl TryFrom<String> for Key {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl Key {
    pub fn is_down(&self, ui: &Ui) -> bool {
        ui.is_key_down(self.key)
            && self.modifiers.iter().all(|modifier| modifier.map(|k| k.is_down(ui)).unwrap_or(true))
            && ModifierState::from(ui) == ModifierState::from(self.modifiers)
    }

    pub fn is_up(&self, ui: &Ui) -> bool {
        !self.is_down(ui)
    }

    pub fn is_pressed(&self, ui: &Ui) -> bool {
        ui.is_key_pressed(self.key)
            && self.modifiers.iter().all(|modifier| modifier.map(|k| k.is_down(ui)).unwrap_or(true))
            && ModifierState::from(ui) == ModifierState::from(self.modifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let key: Key = "ctrl+f".parse().unwrap();
        println!("{key:?}");
    }
}
