pub mod world;

use self::world::World;

pub struct Model {
    world: world::World,
}

impl Model {
    pub fn new() -> Model {
        Model {
            world: world::World::new(),
        }
    }

    pub fn set_character_input(&mut self, id: u64, input: world::character::CharacterInput) {
        self.world.set_character_input(id, input);
    }

    pub fn spawn_character(&mut self) -> u64 {
        self.world.spawn_character()
    }

    pub fn get_world<'a>(&'a self) -> &'a World {
        &self.world
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }
}