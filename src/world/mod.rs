use std::f64::consts::PI;

#[derive(Default, Copy, Clone)]
pub struct PlayerInput {
    flips: u64,
    rotating_right: bool,
    rotating_left: bool,
}

impl PlayerInput {
    pub fn add_flip(&mut self) {
        self.flips += 1
    }

    pub fn set_rotate_right(&mut self, rotating: bool) {
        self.rotating_right = rotating;
    }

    pub fn set_rotate_left(&mut self, rotating: bool) {
        self.rotating_left = rotating;
    }
}

pub struct World {
    angle: f64,
    pub player_input: PlayerInput,
    old_player_input: PlayerInput,
}

impl World {
    pub fn new() -> Self {
        World {
            angle: 0.0,
            player_input: Default::default(),
            old_player_input: Default::default(),
        }
    }

    pub fn tick(&mut self) {
        // flip
        let num_flips = (self.player_input.flips - self.old_player_input.flips) % 2;
        self.angle += num_flips as f64 * PI;

        // rotate
        if self.player_input.rotating_left {
            self.angle += 0.1;
        }
        if self.player_input.rotating_right {
            self.angle -= 0.1
        }

        // make sure angle is between 0 and 2pi
        self.angle = (self.angle % (2.0 * PI) + (2.0 * PI)) % (2.0 * PI);

        // save input for next tick
        self.old_player_input = self.player_input;
    }

    pub fn get_angle(&self) -> f64 {
        self.angle
    }
}