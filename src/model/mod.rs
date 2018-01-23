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

    pub fn get_character_input(&mut self) -> &mut world::character::Input {
        self.world.get_character_input()
    }

    pub fn get_world<'a>(&'a self) -> &'a World {
        &self.world
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }
}