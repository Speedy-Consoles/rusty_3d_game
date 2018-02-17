use std::f64::consts::PI;
use shared::model::world::World;

fn mix(x: f64, y: f64, ratio: f64) -> f64 {
    y * ratio + x * (1.0 - ratio)
}

pub struct VisualCharacter {
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
    pitch: f64,
}

impl VisualCharacter {
    pub fn get_pos(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    pub fn get_yaw(&self) -> f64 {
        self.yaw
    }

    pub fn get_pitch(&self) -> f64 {
        self.pitch
    }
}

pub struct VisualWorld {
    character: VisualCharacter,
}

impl VisualWorld {
    pub fn interpolate(&self, other: &VisualWorld, ratio: f64) -> Self {
        let c1 = &self.character;
        let c2 = &other.character;
        let yaw_diff = c1.yaw - c2.yaw;
        let mut c2_yaw = c2.yaw;
        if yaw_diff > PI {
            c2_yaw += PI * 2.0;
        } else if yaw_diff < -PI {
            c2_yaw -= PI * 2.0;
        }
        let character = VisualCharacter {
            x: mix(c1.x, c2.x, ratio),
            y: mix(c1.y, c2.y, ratio),
            z: mix(c1.z, c2.z, ratio),
            yaw: (mix(c1.yaw, c2_yaw, ratio) + PI * 2.0) % (PI * 2.0),
            pitch: mix(c1.pitch, c2.pitch, ratio),
        };
        VisualWorld {
            character
        }
    }

    pub fn build(current_world: &World, predicted_world: &World) -> Self {
        let wcp = predicted_world.get_character().get_pos();
        let wc_yaw = predicted_world.get_character().get_yaw();
        let wc_pitch = predicted_world.get_character().get_pitch();
        let character = VisualCharacter {
            x: wcp.0,
            y: wcp.1,
            z: wcp.2,
            yaw: wc_yaw,
            pitch: wc_pitch,
        };
        VisualWorld {
            character
        }
    }

    pub fn get_character(&self) -> &VisualCharacter {
        &self.character
    }
}

impl<'a> From<&'a World> for VisualWorld {
    fn from(world: &World) -> Self {
        let wcp = world.get_character().get_pos();
        let wc_yaw = world.get_character().get_yaw();
        let wc_pitch = world.get_character().get_pitch();
        let character = VisualCharacter {
            x: wcp.0,
            y: wcp.1,
            z: wcp.2,
            yaw: wc_yaw,
            pitch: wc_pitch,
        };
        VisualWorld {
            character
        }
    }
}