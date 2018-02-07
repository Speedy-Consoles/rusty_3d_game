use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use shared::consts::CLIENT_CONFIG_FILE;
use super::controls::Controls;
use super::toml;
use shared::ConfigParseError;

#[derive(Default)]
pub struct Config {
    pub controls: Controls,
}

impl Config {
    pub fn load() -> Result<Config, ConfigParseError> {
        use self::toml::value;

        let mut config_file = File::open(CLIENT_CONFIG_FILE)?;

        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string)?;
        let config_value = config_string.parse::<value::Value>()?;
        Config::from_toml(&config_value)
    }

    pub fn save(&self) -> io::Result<()> {
        let mut file = File::create(CLIENT_CONFIG_FILE)?;
        file.write_all(self.to_toml().to_string().as_bytes())
    }

    fn from_toml(value: &toml::value::Value) -> Result<Config, ConfigParseError> {
        use self::toml::value::Value::Table;

        Ok(Config {
            controls: if let &Table(ref map) = value {
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
        use self::toml::value::Value::Table;
        Table(vec![(String::from("controls"), self.controls.to_toml())].into_iter().collect())
    }
}