pub mod character;

use std::collections::HashMap;

use self::character::Character;
use self::character::CharacterInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn set_character_input(&mut self, character_id: u64, input: CharacterInput) {
        if let Some(c) = self.characters.get_mut(&character_id) {
            c.set_input(input);
        }
    }

    pub fn spawn_character(&mut self) -> u64 {
        let character = Character::new();
        self.characters.insert(self.next_character_id, character);
        self.next_character_id += 1;
        self.next_character_id - 1
    }

    pub fn remove_character(&mut self, character_id: u64) {
        if let None = self.characters.remove(&character_id) {
            println!("WARNING: Tried to remove non-existing character with id {}!", character_id);
        }
    }

    pub fn character(&self, character_id: u64) -> Option<&Character> {
        self.characters.get(&character_id)
    }

    pub fn characters(&self) -> &HashMap<u64, Character> {
        &self.characters
    }

    pub fn do_tick(&mut self) {
        for (_, mut c) in self.characters.iter_mut() {
            c.do_tick();
        }
    }
}