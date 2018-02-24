use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use toml;

use shared::consts::CLIENT_CONFIG_FILE;
use shared::ConfigParseError;
use controls::Controls;

pub struct Config {
    pub controls: Controls,
    pub direct_camera: bool,
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

    // TODO holy shit, we need serde
    fn from_toml(value: &toml::Value) -> Result<Config, ConfigParseError> {
        if let &toml::Value::Table(ref map) = value {
            let direct_camera;
            match map.get("graphics") {
                Some(graphics) => {
                    if let &toml::Value::Table(ref map) = graphics {
                        match map.get("DirectCamera") {
                            Some(value) => if let &toml::Value::Boolean(b) = value {
                                direct_camera = b;
                            } else {
                                return Err(ConfigParseError(
                                    String::from("DirectCamera is not a Boolean!")));
                            },
                            None => return Err(ConfigParseError(
                                String::from("DirectCamera not in config!"))),
                        }
                    } else {
                        return Err(ConfigParseError(String::from("Graphics is not a table!")))
                    }
                },
                None => return Err(ConfigParseError(
                    String::from("No graphics section in config!")))
            }

            let config = Config {
                controls: match map.get("controls") {
                    Some(value) => Controls::from_toml(value)?,
                    None => return Err(ConfigParseError(
                        String::from("No controls section in config!")))
                },
                direct_camera,
            };
            Ok(config)
        } else {
            return Err(ConfigParseError(String::from("Config is not a table!")))
        }
    }

    pub fn to_toml(&self) -> toml::value::Value {
        toml::Value::Table(vec![
            (String::from("controls"), self.controls.to_toml()),
            (String::from("graphics"), toml::Value::Table(vec![
                (String::from("DirectCamera"), toml::Value::Boolean(self.direct_camera))
            ].into_iter().collect()))
        ].into_iter().collect())
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            controls: Default::default(),
            direct_camera: true,
        }
    }
}