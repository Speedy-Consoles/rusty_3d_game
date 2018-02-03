use std::convert::AsRef;

use super::toml;
use super::NumCast;

use super::ParseError;
use super::MouseWheelDirection;
use super::PushButton;
use super::PushButtonState;

#[derive(Debug)]
pub enum FireTrigger {
    Button(SwitchTrigger),
    MouseWheelTick(MouseWheelDirection),
}

impl FireTrigger {
    pub fn from_toml(value: &toml::value::Value) -> Result<FireTrigger, ParseError> {
        use self::toml::value::Value::*;
        use self::FireTrigger::*;
        use self::MouseWheelDirection::*;

        if let Ok(switch_trigger) = SwitchTrigger::from_toml(value) {
            Ok(Button(switch_trigger))
        } else {
            match value {
                &String(ref s) => match s.as_ref() {
                    "MouseWheelUp" => Ok(MouseWheelTick(Up)),
                    "MouseWheelDown" => Ok(MouseWheelTick(Down)),
                    _ => Err(ParseError(format!("Unknown fire trigger: '{}'", s))),
                }
                _ => Err(ParseError(format!("Fire trigger must be string, got '{}'!", value))),
            }
        }
    }
}

#[derive(Debug)]
pub struct SwitchTrigger {
    pub button: PushButton,
    pub state: PushButtonState
}

impl SwitchTrigger {
    pub fn from_toml(value: &toml::value::Value) -> Result<SwitchTrigger, ParseError> {
        use self::toml::value::Value::*;
        use super::PushButtonState::*;

        match value {
            &Table(ref t) => {
                let state = match t.get("inverted") {
                    Some(&Boolean(true)) => Released,
                    Some(&Boolean(false)) => Pressed,
                    Some(v) =>
                        return Err(ParseError(format!("'invalid' must be boolean, got '{}'!", v))),
                    None => Pressed,
                };
                let button = match t.get("button") {
                    Some(button_value) => PushButton::from_toml(button_value)?,
                    None => return Err(ParseError("No button specified!".into())),
                };
                Ok(SwitchTrigger { button, state })
            },
            _ => Ok(SwitchTrigger { button: PushButton::from_toml(value)?, state: Pressed })
        }
    }
}

#[derive(Debug)]
pub enum ValueTrigger {
    MouseWheel(f64),
    // TODO add mouse
    Axis { axis: u32, factor: f64 },
}

impl ValueTrigger {
    pub fn from_toml(value: &toml::value::Value) -> Result<ValueTrigger, ParseError> {
        use self::toml::value::Value::*;
        use self::ValueTrigger::*;

        let axis_value;
        let mut factor;
        match value {
            &Table(ref t) => {
                axis_value = match t.get("axis") {
                    Some(v) => v,
                    None => return Err(ParseError("No axis specified!".into())),
                };
                factor = match t.get("sensitivity") {
                    Some(&Float(f)) => f,
                    Some(v) => return Err(ParseError(
                        format!("'sensitivity' must be float, got '{}'!", v))),
                    None => 1.0,
                };
                match t.get("inverted") {
                    Some(&Boolean(true)) => factor *= -1.0,
                    Some(&Boolean(false)) => (),
                    Some(v) =>
                        return Err(ParseError(format!("'inverted' must be bool, got '{}'!", v))),
                    None => (),
                };
            },
            v => {
                axis_value = v;
                factor = 1.0;
            },
        }
        match axis_value {
            &Integer(i) => match NumCast::from(i) {
                Some(axis) => Ok(Axis { axis, factor }),
                None => return Err(ParseError(format!("Invalid axis id: {}", i))),
            },
            &String(ref s) => match s.as_ref() {
                "MouseWheel" => Ok(MouseWheel(factor)),
                _ => Err(ParseError(format!("Unknown axis: '{}'", s))),
            }
            v => Err(ParseError(format!("'axis' must be integer or string, got '{}'!", v))),
        }
    }
}