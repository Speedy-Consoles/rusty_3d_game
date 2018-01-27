pub mod character;

use self::character::Character;
use self::character::CharacterInput as CharacterInput;

pub struct World {
    character: Character,
}

impl World {
    pub fn new() -> Self {
        World {
            character: Character::new(),
        }
    }

    pub fn set_character_input(&mut self, input: CharacterInput) {
        self.character.set_input(input);
    }

    pub fn get_character(&self) -> &Character {
        &self.character
    }

    pub fn tick(&mut self) {
        self.character.tick();
    }
}