pub mod character;

use std::collections::HashMap;

use self::character::Character;
use self::character::CharacterInput;

#[derive(Clone)]
pub struct World {
    characters: HashMap<u64, Character>,
    next_character_id: u64,
}

impl World {
    pub fn new() -> Self {
        World {
            characters: HashMap::new(),
            next_character_id: 0,
        }
    }

    pub fn set_character_input(&mut self, id: u64, input: CharacterInput) {
        if let Some(c) = self.characters.get_mut(&id) {
            c.set_input(input);
        }
    }

    pub fn spawn_character(&mut self) -> u64 {
        let character = Character::new();
        self.characters.insert(self.next_character_id, character);
        self.next_character_id += 1;
        self.next_character_id - 1
    }

    pub fn get_characters(&self) -> &HashMap<u64, Character> {
        &self.characters
    }

    pub fn tick(&mut self) {
        for (_, mut c) in self.characters.iter_mut() {
            c.tick();
        }
    }
}