use math::FixedPoint;
use math::FPAngle;
use math::Vec3;
//use consts::WALKING_SPEED; // TODO

#[derive(Default, Copy, Clone)]
pub struct CharacterInput {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    pub jumping: bool, // TODO consider to make this a counter, so we don't need to reset it
    yaw: FPAngle,
    pitch: FPAngle,
}

impl CharacterInput {
    pub fn add_yaw(&mut self, delta: FPAngle) {
        let w = FPAngle::whole();
        self.yaw = ((self.yaw + delta) % w + w) % w;
    }

    pub fn add_pitch(&mut self, delta: FPAngle) {
        let q = FPAngle::quarter();
        self.pitch = (self.pitch + delta).max(-q).min(q);
    }

    pub fn get_yaw(&self) -> FPAngle {
        self.yaw
    }

    pub fn get_pitch(&self) -> FPAngle {
        self.pitch
    }

    pub fn reset_flags(&mut self) {
        self.jumping = false;
    }
}

#[derive(Clone)]
pub struct Character {
    pos: Vec3,
    yaw: FPAngle,
    pitch: FPAngle,
    input: CharacterInput,
}

impl Character {
    pub fn new() -> Character {
        Character {
            input: Default::default(),
            pos: Vec3::new(
                FixedPoint::new(0),
                FixedPoint::new(0),
                FixedPoint::fraction(7, 10)
            ),
            yaw: FPAngle::zero(),
            pitch: FPAngle::zero(),
        }
    }

    pub fn set_input(&mut self, input: CharacterInput) {
        self.input = input;
    }

    pub fn get_pos(&self) -> Vec3 {
        self.pos
    }

    pub fn get_yaw(&self) -> FPAngle {
        self.yaw
    }

    pub fn get_pitch(&self) -> FPAngle {
        self.pitch
    }

    pub fn tick(&mut self) {
        let mut dir = Vec3::zero();
        if self.input.forward {
            dir.x += 1.into();
        }
        if self.input.backward {
            dir.x -= 1.into();
        }
        if self.input.right {
            dir.y -= 1.into();
        }
        if self.input.left {
            dir.y += 1.into();
        }

        let ys = self.yaw.sin();
        let yc = self.yaw.cos();
        dir = Vec3::new(
            dir.x * yc - dir.y * ys,
            dir.x * ys + dir.y * yc,
            dir.z
        );

        if !dir.is_zero() {
            let walking_speed = FixedPoint::fraction(1, 10); // TODO use const in consts instead
            dir = dir.scale_to(walking_speed.into());
        }

        self.pos += dir;

        self.yaw = self.input.yaw;
        self.pitch = self.input.pitch;
    }
}