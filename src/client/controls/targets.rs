use std::str::FromStr;

use super::ParseError;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, EnumString)]
pub enum FireTarget {
    Jump,
    NextWeapon,
    PrevWeapon,
    Exit,
    ToggleMenu,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, EnumString)]
pub enum SwitchTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
    Shoot,
    Aim,
}

// TODO add invert attribute
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, EnumString)]
pub enum ValueTarget {
    Yaw,
    Pitch,
}

#[derive(Debug)]
pub enum Target {
    Fire(FireTarget),
    Switch(SwitchTarget),
    Value(ValueTarget),
}

impl FromStr for Target {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::Target::*;
        if let Ok(target) = s.parse::<FireTarget>() {
            Ok(Fire(target))
        } else if let Ok(target) = s.parse::<SwitchTarget>() {
            Ok(Switch(target))
        } else if let Ok(target) = s.parse::<ValueTarget>() {
            Ok(Value(target))
        } else {
            Err(ParseError(format!("Unknown target '{}'!", s)))
        }
    }
}