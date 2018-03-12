pub mod world;
pub mod player;

use std::collections::HashMap;

use self::player::Player;
use self::world::World;
use self::world::character::CharacterInput;

// TODO maybe replace ids with weak references?

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    players: HashMap<u64, Player>,
    world: World,
    next_player_id: u64,
}

impl Model {
    pub fn new() -> Model {
        Model {
            players: HashMap::new(),
            world: world::World::new(),
            next_player_id: 0,
        }
    }

    pub fn set_character_input(&mut self, player_id: u64, input: CharacterInput) {
        if let Some(character_id) = self.players.get(&player_id).unwrap().character_id() {
            self.world.set_character_input(character_id, input);
        }
    }

    pub fn add_player(&mut self, name: String) -> u64 {
        let id = self.next_player_id;
        let character_id = self.world.spawn_character();
        let mut player = Player::new(name);
        player.set_character_id(Some(character_id));
        self.players.insert(id, player);
        self.next_player_id += 1;
        id
    }

    pub fn remove_player(&mut self, player_id: u64) -> Option<Player> {
        let mut result = self.players.remove(&player_id);
        if let Some(ref mut player) = result {
            if let Some(character_id) = player.character_id() {
                self.world.remove_character(character_id);
                player.set_character_id(None);
            }
        }
        result
    }

    pub fn player(&self, player_id: u64) -> Option<&Player> {
        self.players.get(&player_id)
    }

    pub fn world<'a>(&'a self) -> &'a World {
        &self.world
    }

    pub fn do_tick(&mut self) {
        self.world.do_tick();
    }
}