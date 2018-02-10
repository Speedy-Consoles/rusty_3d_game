use std::f64::consts::PI;

#[derive(Default, Copy, Clone)]
pub struct CharacterInput {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    pub jumping: bool, // TODO consider to make this a counter, so we don't need to reset it
    yaw: f64,
    pitch: f64,
}

impl CharacterInput {
    pub fn add_yaw(&mut self, delta: f64) {
        self.yaw = ((self.yaw + delta) % (PI * 2.0) + (PI * 2.0)) % (PI * 2.0);
    }

    pub fn add_pitch(&mut self, delta: f64) {
        self.pitch = (self.pitch + delta).max(-PI / 2.0).min(PI / 2.0);
    }

    pub fn get_yaw(&self) -> f64 {
        self.yaw
    }

    pub fn get_pitch(&self) -> f64 {
        self.pitch
    }

    pub fn reset_flags(&mut self) {
        self.jumping = false;
    }
}

#[derive(Clone)]
pub struct Character {
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
    pitch: f64,
    input: CharacterInput,
}

impl Character {
    pub fn new() -> Character {
        Character {
            input: Default::default(),
            x: 0.0,
            y: 0.0,
            z: 0.7,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn set_input(&mut self, input: CharacterInput) {
        self.input = input;
    }

    pub fn get_pos(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    pub fn get_yaw(&self) -> f64 {
        self.yaw
    }

    pub fn get_pitch(&self) -> f64 {
        self.pitch
    }

    pub fn tick(&mut self) {
        if self.input.forward {
            self.x += self.yaw.cos() * 0.1;
            self.y += self.yaw.sin() * 0.1;
        }
        if self.input.backward {
            self.x -= self.yaw.cos() * 0.1;
            self.y -= self.yaw.sin() * 0.1;
        }
        if self.input.right {
            self.x += self.yaw.sin() * 0.1;
            self.y -= self.yaw.cos() * 0.1;
        }
        if self.input.left {
            self.x -= self.yaw.sin() * 0.1;
            self.y += self.yaw.cos() * 0.1;
        }
        self.yaw = self.input.yaw;
        self.pitch = self.input.pitch;
    }
}