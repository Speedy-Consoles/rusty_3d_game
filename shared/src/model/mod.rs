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

    pub fn set_character_input(&mut self, input: world::character::CharacterInput) {
        self.world.set_character_input(input);
    }

    pub fn get_world<'a>(&'a self) -> &'a World {
        &self.world
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }
}