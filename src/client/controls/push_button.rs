use std::convert::AsRef;

use super::VirtualKeyCode;
use super::VirtualKeyCode::*;
use super::num::cast::NumCast;
use super::glutin;
use super::toml;
use super::ParseError;

const KEY_CODE_PAIRS: &'static [(VirtualKeyCode, &'static str)] = &[
    (Key1, "1"),
    (Key2, "2"),
    (Key3, "3"),
    (Key4, "4"),
    (Key5, "5"),
    (Key6, "6"),
    (Key7, "7"),
    (Key8, "8"),
    (Key9, "9"),
    (Key0, "0"),
    (A, "A"),
    (B, "B"),
    (C, "C"),
    (D, "D"),
    (E, "E"),
    (F, "F"),
    (G, "G"),
    (H, "H"),
    (I, "I"),
    (J, "J"),
    (K, "K"),
    (L, "L"),
    (M, "M"),
    (N, "N"),
    (O, "O"),
    (P, "P"),
    (Q, "Q"),
    (R, "R"),
    (S, "S"),
    (T, "T"),
    (U, "U"),
    (V, "V"),
    (W, "W"),
    (X, "X"),
    (Y, "Y"),
    (Z, "Z"),
    (Escape, "Escape"),
    (F1, "F1"),
    (F2, "F2"),
    (F3, "F3"),
    (F4, "F4"),
    (F5, "F5"),
    (F6, "F6"),
    (F7, "F7"),
    (F8, "F8"),
    (F9, "F9"),
    (F10, "F10"),
    (F11, "F11"),
    (F12, "F12"),
    (F13, "F13"),
    (F14, "F14"),
    (F15, "F15"),
    (Snapshot, "Snapshot"),
    (Scroll, "Scroll"),
    (Pause, "Pause"),
    (Insert, "Insert"),
    (Home, "Home"),
    (Delete, "Delete"),
    (End, "End"),
    (PageDown, "PageDown"),
    (PageUp, "PageUp"),
    (Left, "Left"),
    (Up, "Up"),
    (Right, "Right"),
    (Down, "Down"),
    (Back, "Back"),
    (Return, "Return"),
    (Space, "Space"),
    (Compose, "Compose"),
    (Numlock, "Numlock"),
    (Numpad0, "Numpad0"),
    (Numpad1, "Numpad1"),
    (Numpad2, "Numpad2"),
    (Numpad3, "Numpad3"),
    (Numpad4, "Numpad4"),
    (Numpad5, "Numpad5"),
    (Numpad6, "Numpad6"),
    (Numpad7, "Numpad7"),
    (Numpad8, "Numpad8"),
    (Numpad9, "Numpad9"),
    (AbntC1, "AbntC1"),
    (AbntC2, "AbntC2"),
    (Add, "Add"),
    (Apostrophe, "Apostrophe"),
    (Apps, "Apps"),
    (At, "At"),
    (Ax, "Ax"),
    (Backslash, "Backslash"),
    (Calculator, "Calculator"),
    (Capital, "Capital"),
    (Colon, "Colon"),
    (Comma, "Comma"),
    (Convert, "Convert"),
    (Decimal, "Decimal"),
    (Divide, "Divide"),
    (Equals, "Equals"),
    (Grave, "Grave"),
    (Kana, "Kana"),
    (Kanji, "Kanji"),
    (LAlt, "LAlt"),
    (LBracket, "LBracket"),
    (LControl, "LControl"),
    (LMenu, "LMenu"),
    (LShift, "LShift"),
    (LWin, "LWin"),
    (Mail, "Mail"),
    (MediaSelect, "MediaSelect"),
    (MediaStop, "MediaStop"),
    (Minus, "Minus"),
    (Multiply, "Multiply"),
    (Mute, "Mute"),
    (MyComputer, "MyComputer"),
    (NavigateForward, "NavigateForward"),
    (NavigateBackward, "NavigateBackward"),
    (NextTrack, "NextTrack"),
    (NoConvert, "NoConvert"),
    (NumpadComma, "NumpadComma"),
    (NumpadEnter, "NumpadEnter"),
    (NumpadEquals, "NumpadEquals"),
    (OEM102, "OEM102"),
    (Period, "Period"),
    (PlayPause, "PlayPause"),
    (Power, "Power"),
    (PrevTrack, "PrevTrack"),
    (RAlt, "RAlt"),
    (RBracket, "RBracket"),
    (RControl, "RControl"),
    (RMenu, "RMenu"),
    (RShift, "RShift"),
    (RWin, "RWin"),
    (Semicolon, "Semicolon"),
    (Slash, "Slash"),
    (Sleep, "Sleep"),
    (Stop, "Stop"),
    (Subtract, "Subtract"),
    (Sysrq, "Sysrq"),
    (Tab, "Tab"),
    (Underline, "Underline"),
    (Unlabeled, "Unlabeled"),
    (VolumeDown, "VolumeDown"),
    (VolumeUp, "VolumeUp"),
    (Wake, "Wake"),
    (WebBack, "WebBack"),
    (WebFavorites, "WebFavorites"),
    (WebForward, "WebForward"),
    (WebHome, "WebHome"),
    (WebRefresh, "WebRefresh"),
    (WebSearch, "WebSearch"),
    (WebStop, "WebStop"),
    (Yen, "Yen"),
];

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum PushButton {
    ScanCode(u32),
    KeyCode(VirtualKeyCode),
    MouseButton(glutin::MouseButton),
}

impl PushButton {
    pub fn from_toml(value: &toml::value::Value) -> Result<PushButton, ParseError> {
        use super::toml::value::Value::*;
        use super::glutin::MouseButton::*;
        use super::PushButton::*;

        match value {
            &Integer(i) => match NumCast::from(i) {
                Some(sc) => Ok(ScanCode(sc)),
                None => return Err(ParseError(format!("Invalid scan code: {}", i))),
            },
            &String(ref s) => {
                match s.as_ref() {
                    "MouseLeft" => Ok(MouseButton(Left)),
                    "MouseRight" => Ok(MouseButton(Right)),
                    "MouseMiddle" => Ok(MouseButton(Middle)),
                    ss => {
                        if ss.starts_with("Mouse") {
                            match ss[5..].parse() {
                                Ok(i) => Ok(MouseButton(Other(i))),
                                Err(_) => Err(ParseError(format!("Unknown push button {}", s))),
                            }
                        } else {
                            for &(kc, name) in KEY_CODE_PAIRS {
                                if name == ss {
                                    return Ok(KeyCode(kc));
                                }
                            }
                            Err(ParseError(format!("Unknown push button {}", s)))
                        }
                    }
                }
            }
            _ => Err(ParseError(format!("Unknown push button {}", *value)))
        }
    }
}