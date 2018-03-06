pub mod consts;
pub mod math;
pub mod util;
pub mod model;
pub mod net;

#[macro_use] extern crate custom_derive;
#[macro_use] extern crate newtype_derive;
extern crate toml;
extern crate cgmath;
extern crate serde;
extern crate bincode;
#[macro_use] extern crate serde_derive;

use std::fmt;
use std::io;

#[derive(Debug)]
pub struct ConfigParseError(pub String);

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<io::Error> for ConfigParseError {
    fn from(err: io::Error) -> Self {
        ConfigParseError(err.to_string())
    }
}

impl From<toml::de::Error> for ConfigParseError {
    fn from(err: toml::de::Error) -> Self {
        ConfigParseError(err.to_string())
    }
}

