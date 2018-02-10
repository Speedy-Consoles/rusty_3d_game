use std::f64::consts::PI;
use shared::model::world::World;

fn mix(x: f64, y: f64, ratio: f64) -> f64 {
    y * ratio + x * (1.0 - ratio)
}

pub struct InterpolateCharacter {
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
    pitch: f64,
}

impl InterpolateCharacter {
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

pub struct InterpolateWorld {
    character: InterpolateCharacter,
}

impl InterpolateWorld {
    pub fn interpolate(&self, world: &World, ratio: f64) -> Self {
        let wcp = world.get_character().get_pos();
        let mut wc_yaw = world.get_character().get_yaw();
        let wc_pitch = world.get_character().get_pitch();
        let ic = &self.character;
        let yaw_diff = ic.yaw - wc_yaw;
        if yaw_diff > PI {
            wc_yaw += PI * 2.0;
        } else if yaw_diff < -PI {
            wc_yaw -= PI * 2.0;
        }
        let character = InterpolateCharacter {
            x: mix(ic.x, wcp.0, ratio),
            y: mix(ic.y, wcp.1, ratio),
            z: mix(ic.z, wcp.2, ratio),
            yaw: (mix(ic.yaw, wc_yaw, ratio) + PI * 2.0) % (PI * 2.0),
            pitch: mix(ic.pitch, wc_pitch, ratio),
        };
        InterpolateWorld {
            character
        }
    }

    pub fn get_character(&self) -> &InterpolateCharacter {
        &self.character
    }
}

impl<'a> From<&'a World> for InterpolateWorld {
    fn from(world: &World) -> Self {
        let wcp = world.get_character().get_pos();
        let wc_yaw = world.get_character().get_yaw();
        let wc_pitch = world.get_character().get_pitch();
        let character = InterpolateCharacter {
            x: wcp.0,
            y: wcp.1,
            z: wcp.2,
            yaw: wc_yaw,
            pitch: wc_pitch,
        };
        InterpolateWorld {
            character
        }
    }
}