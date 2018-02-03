use super::toml;
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
        // TODO
        Err(ParseError("Not yet implemented".into()))
    }
}

#[derive(Debug)]
pub struct SwitchTrigger {
    pub button: PushButton,
    pub state: PushButtonState
}

impl SwitchTrigger {
    pub fn from_toml(value: &toml::value::Value) -> Result<SwitchTrigger, super::ParseError> {
        // TODO
        Err(ParseError("Not yet implemented".into()))
    }
}

#[derive(Debug)]
pub enum ValueTrigger {
    MouseWheel(f64),
    Axis { axis: u32, factor: f64 },
}

impl ValueTrigger {
    pub fn from_toml(value: &toml::value::Value) -> Result<ValueTrigger, super::ParseError> {
        // TODO
        Err(ParseError("Not yet implemented".into()))
    }
}