#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn name(&mut self) -> &str {
        self.name.as_str()
    }

    pub fn set_character_id(&mut self, character_id: Option<u64>) {
        self.character_id = character_id;
    }

    pub fn character_id(&self) -> Option<u64> {
        self.character_id
    }

    pub fn take_name(self) -> String {
        self.name
    }
}