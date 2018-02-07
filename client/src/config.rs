use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use toml;

use shared::consts::CLIENT_CONFIG_FILE;
use shared::ConfigParseError;
use controls::Controls;

#[derive(Default)]
pub struct Config {
    pub controls: Controls,
}

impl Config {
    pub fn load() -> Result<Config, ConfigParseError> {
        let mut config_file = File::open(CLIENT_CONFIG_FILE)?;

        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string)?;
        let config_value = config_string.parse::<toml::Value>()?;
        Config::from_toml(&config_value)
    }

    pub fn save(&self) -> io::Result<()> {
        let mut file = File::create(CLIENT_CONFIG_FILE)?;
        file.write_all(self.to_toml().to_string().as_bytes())
    }

    fn from_toml(value: &toml::Value) -> Result<Config, ConfigParseError> {
        Ok(Config {
            controls: if let &toml::Value::Table(ref map) = value {
                match map.get("controls") {
                    Some(value) => Controls::from_toml(value)?,
                    None => return Err(ConfigParseError(String::from("Config is not a table!")))
                }
            } else {
                return Err(ConfigParseError(String::from("Config is not a table!")))
            }
        })
    }

    pub fn to_toml(&self) -> toml::value::Value {
        toml::Value::Table(vec![(String::from("controls"), self.controls.to_toml())].into_iter().collect())
    }
}