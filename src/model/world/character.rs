use std::f64::consts::PI;

#[derive(Default, Copy, Clone)]
pub struct Input {
    jumping: bool,
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    yaw: f64,
    pitch: f64,
}

impl Input {
    pub fn jump(&mut self) {
        self.jumping = true;
    }

    pub fn set_yaw (&mut self, yaw: f64) {
        self.yaw = (yaw % (PI * 2.0) + (PI * 2.0)) % (PI * 2.0);
    }

    pub fn set_pitch (&mut self, pitch: f64) {
        self.pitch = if pitch < -PI / 2.0 {
            -PI / 2.0
        } else if pitch > PI / 2.0 {
            PI / 2.0
        } else {
            pitch
        };
    }

    fn reset_triggers(&mut self) {
        self.jumping = false;
    }
}

pub struct Character {
    input: Input,
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
    pitch: f64,
}

impl Character {
    pub fn new() -> Character {
        Character {
            input: Default::default(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn get_input<'a>(&'a mut self) -> &'a mut Input {
        &mut self.input
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

        // reset triggers, so they don't occur again
        self.input.reset_triggers();
    }
}