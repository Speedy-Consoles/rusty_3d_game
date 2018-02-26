use std::str::FromStr;

use shared::ConfigParseError;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ToString, EnumString)]
pub enum FireTarget {
    Jump,
    NextWeapon,
    PrevWeapon,
    Exit,
    ToggleMenu,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, ToString, EnumString)]
pub enum SwitchTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
    Crouch,
    Shoot,
    Aim,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, ToString, EnumString, EnumIter)]
pub enum ValueTarget {
    Yaw,
    Pitch,
}

impl ValueTarget {
    pub fn get_base_factor(&self) -> f64 {
        use self::ValueTarget::*;

        match *self {
            Yaw => -0.0002,
            Pitch => -0.0002,
        }
    }
}

#[derive(Debug)]
pub enum Target {
    Fire(FireTarget),
    Switch(SwitchTarget),
    Value(ValueTarget),
}

impl FromStr for Target {
    type Err = ConfigParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::Target::*;

        if let Ok(target) = s.parse::<FireTarget>() {
            Ok(Fire(target))
        } else if let Ok(target) = s.parse::<SwitchTarget>() {
            Ok(Switch(target))
        } else if let Ok(target) = s.parse::<ValueTarget>() {
            Ok(Value(target))
        } else {
            Err(ConfigParseError(format!("Unknown target '{}'!", s)))
        }
    }
}