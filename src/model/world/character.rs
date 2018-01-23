
#[derive(Default, Copy, Clone)]
pub struct Input {
    jumping: bool,
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
}

impl Input {
    pub fn jump(&mut self) {
        self.jumping = true;
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
}

impl Character {
    pub fn new() -> Character {
        Character {
            input: Default::default(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn get_input<'a>(&'a mut self) -> &'a mut Input {
        &mut self.input
    }

    pub fn get_pos(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    pub fn tick(&mut self) {
        if self.input.right {
            self.y -= 0.1;
        }
        if self.input.left {
            self.y += 0.1;
        }
        if self.input.forward {
            self.x += 0.1;
        }
        if self.input.backward {
            self.x -= 0.1;
        }

        // reset triggers, so they don't occur again
        self.input.reset_triggers();
    }
}