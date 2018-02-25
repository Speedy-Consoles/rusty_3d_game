use std::f32::consts::PI;

use cgmath::Vector3;

use shared::model::world::World;

pub trait Mix {
    fn mix(&self, other: &Self, ratio: f32) -> Self;
}

impl Mix for f32 {
    fn mix(&self, other: &f32, ratio: f32) -> Self {
        *self * (1.0 - ratio) + *other * ratio
    }
}

impl Mix for Vector3<f32> {
    fn mix(&self, other: &Self, ratio: f32) -> Self {
        Vector3::new(
            self.x.mix(&other.x, ratio),
            self.y.mix(&other.y, ratio),
            self.z.mix(&other.z, ratio)
        )
    }
}

#[derive(Clone)]
pub struct VisualCharacter {
    pos: Vector3<f32>,
    yaw: f32,
    pitch: f32,
}

impl VisualCharacter {
    pub fn get_pos(&self) -> Vector3<f32> {
        self.pos
    }

    pub fn get_yaw(&self) -> f32 {
        self.yaw
    }

    pub fn get_pitch(&self) -> f32 {
        self.pitch
    }
}

impl Mix for VisualCharacter {
    fn mix(&self, other: &Self, ratio: f32) -> Self {
        let yaw_diff = self.yaw - other.yaw;
        let mut oy = other.yaw;
        if yaw_diff > PI {
            oy += PI * 2.0;
        } else if yaw_diff < -PI {
            oy -= PI * 2.0;
        }
        VisualCharacter {
            pos: self.pos.mix(&other.pos, ratio),
            yaw: self.yaw.mix(&oy, ratio),
            pitch: self.pitch.mix(&other.pitch, ratio),
        }
    }
}

#[derive(Clone)]
pub struct VisualWorld {
    character: VisualCharacter,
}

impl VisualWorld {
    pub fn build(current_world: &World, predicted_world: &World) -> Self {
        let wcp = predicted_world.get_character().get_pos();
        let wc_yaw = predicted_world.get_character().get_view_dir().get_yaw();
        let wc_pitch = predicted_world.get_character().get_view_dir().get_pitch();
        let character = VisualCharacter {
            pos: wcp.into(),
            yaw: wc_yaw.rad_f32(),
            pitch: wc_pitch.rad_f32(),
        };
        VisualWorld {
            character
        }
    }

    pub fn get_character(&self) -> &VisualCharacter {
        &self.character
    }
}

impl Mix for VisualWorld {
    fn mix(&self, other: &VisualWorld, ratio: f32) -> Self {
        VisualWorld {
            character: self.character.mix(&other.character, ratio)
        }
    }
}

impl<'a> From<&'a World> for VisualWorld {
    fn from(world: &World) -> Self {
        let c = world.get_character();
        let character = VisualCharacter {
            pos: c.get_pos().into(),
            yaw: c.get_view_dir().get_yaw().rad_f32(),
            pitch: c.get_view_dir().get_pitch().rad_f32(),
        };
        VisualWorld {
            character
        }
    }
}