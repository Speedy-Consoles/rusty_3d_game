use std::f32::consts::PI;
use std::collections::HashMap;

use cgmath::Vector3;

use shared::util::Mix;
use shared::model::world::World;
use shared::model::world::character::Character;

#[derive(Clone)]
pub struct VisualCharacter {
    pos: Vector3<f32>,
    yaw: f32,
    pitch: f32,
}

impl VisualCharacter {
    pub fn build(character: &Character) -> VisualCharacter {
        let wcp = character.pos();
        let wc_yaw = character.view_dir().yaw();
        let wc_pitch = character.view_dir().pitch();
        VisualCharacter {
            pos: wcp.into(),
            yaw: wc_yaw.rad_f32(),
            pitch: wc_pitch.rad_f32(),
        }
    }

    pub fn pos(&self) -> Vector3<f32> {
        self.pos
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }
}

impl Mix for VisualCharacter {
    fn mix(&self, other: &Self, ratio: f64) -> Self {
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
    characters: HashMap<u64, VisualCharacter>,
}

impl VisualWorld {
    pub fn new() -> VisualWorld {
        VisualWorld {
            characters: HashMap::new(),
        }
    }

    pub fn rebuild(&mut self, my_character_id: Option<u64>,
                   current_world: &World, predicted_world: &World) {
        self.reset();
        for (&id, c) in  current_world.characters() {
            if Some(id) == my_character_id {
                continue;
            }
            self.characters.insert(id, VisualCharacter::build(c));
        }
        if let Some(id) = my_character_id {
            if let Some(character) = predicted_world.character(id) {
                self.characters.insert(id, VisualCharacter::build(character));
            }
        }
    }

    pub fn remix(&mut self, a: &VisualWorld, b: &VisualWorld, ratio: f64) {
        self.reset();
        for (id, cb) in b.characters() {
            if let Some(ca) = a.characters().get(id) {
                self.characters.insert(*id, ca.mix(cb, ratio));
            } else {
                self.characters.insert(*id, cb.clone()); // always insert characters of the current world
            }
        }
    }

    pub fn character(&self, character_id: u64) -> Option<&VisualCharacter> {
        self.characters.get(&character_id)
    }

    pub fn characters(&self) -> &HashMap<u64, VisualCharacter> {
        &self.characters
    }

    fn reset(&mut self) {
        self.characters.clear();
    }
}