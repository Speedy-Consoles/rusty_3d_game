pub struct Player {
    name: String,
    character_id: Option<u64>,
}

impl Player {
    pub fn new(name: String) -> Player {
        Player {
            name,
            character_id: None,
        }
    }

    pub fn rename(&mut self, new_name: String) {
        self.name = new_name;
    }

    pub fn set_character_id(&mut self, character_id: Option<u64>) {
        self.character_id = character_id;
    }

    pub fn get_character_id(&self) -> Option<u64> {
        self.character_id
    }
}