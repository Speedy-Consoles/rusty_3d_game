use std::fmt;
pub mod consts;

#[derive(Debug)]
pub struct ConfigParseError(pub String);

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}