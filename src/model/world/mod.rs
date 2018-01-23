pub mod character;

use self::character::Character;
use self::character::Input as CharacterInput;

pub struct World {
    character: Character,
}

impl World {
    pub fn new() -> Self {
        World {
            character: Character::new(),
        }
    }

    pub fn get_character_input(&mut self) -> &mut CharacterInput {
        self.character.get_input()
    }

    pub fn get_character(&self) -> &Character {
        &self.character
    }

    pub fn tick(&mut self) {
        self.character.tick();
    }
}