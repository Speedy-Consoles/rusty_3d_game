use std::str::FromStr;

use super::ParseError;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FireTarget {
    Jump,
    NextWeapon,
    PrevWeapon,
    Exit,
    ToggleMenu,
}

impl FromStr for FireTarget {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::FireTarget::*;
        match &*s.to_lowercase() {
            "jump" => Ok(Jump),
            "exit" => Ok(Exit),
            "togglemenu" => Ok(ToggleMenu),
            _ => Err(ParseError(format!("Unknown fire target '{}'", s))),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwitchTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
    Shoot,
    Aim,
}

impl FromStr for SwitchTarget {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::SwitchTarget::*;
        match &*s.to_lowercase() {
            "moveright" => Ok(MoveRight),
            "moveleft" => Ok(MoveLeft),
            "moveforward" => Ok(MoveForward),
            "movebackward" => Ok(MoveBackward),
            "shoot" => Ok(Shoot),
            _ => Err(ParseError(format!("Unknown switch target '{}'", s))),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueTarget {
    Yaw,
    Pitch,
}

impl FromStr for ValueTarget {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::ValueTarget::*;
        match &*s.to_lowercase() {
            "yaw" => Ok(Yaw),
            "pitch" => Ok(Pitch),
            _ => Err(ParseError(format!("Unknown value target '{}'", s))),
        }
    }
}