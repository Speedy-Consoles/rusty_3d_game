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
    vel: Vec3,
    yaw: FPAngle,
    pitch: FPAngle,
    input: CharacterInput,
}

impl Character {
    pub fn new() -> Character {
        let character_height = FixedPoint::fraction(17, 10); // TODO use const in consts instead
        Character {
            input: Default::default(),
            pos: Vec3::new(
                FixedPoint::new(0),
                FixedPoint::new(0),
                character_height,
            ),
            vel: Vec3::zero(),
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
        let mut input_acceleration = Vec3::zero();
        if self.input.forward {
            input_acceleration.x += 1.into();
        }
        if self.input.backward {
            input_acceleration.x -= 1.into();
        }
        if self.input.right {
            input_acceleration.y -= 1.into();
        }
        if self.input.left {
            input_acceleration.y += 1.into();
        }

        let ys = self.yaw.sin();
        let yc = self.yaw.cos();
        input_acceleration = Vec3::new(
            input_acceleration.x * yc - input_acceleration.y * ys,
            input_acceleration.x * ys + input_acceleration.y * yc,
            input_acceleration.z
        );

        // TODO move these to consts
        let character_height = FixedPoint::fraction(17, 10);
        let ground_acceleration = FixedPoint::fraction(1, 20);
        let air_acceleration = FixedPoint::fraction(1, 200);
        let max_walking_speed = FixedPoint::fraction(1, 10);
        let ground_friction = FixedPoint::one() + ground_acceleration / max_walking_speed;
        let air_friction = FixedPoint::fraction(100, 95);
        let jump_velocity = FixedPoint::fraction(1, 3);
        let gravity = FixedPoint::fraction(1, 50);

        if !input_acceleration.is_zero() {
            input_acceleration = if self.grounded() {
                input_acceleration.scale_to(ground_acceleration)
            } else {
                input_acceleration.scale_to(air_acceleration)
            };
        }

        if self.grounded() && self.input.jumping {
            input_acceleration.z += jump_velocity;
        }

        println!("{:?}", input_acceleration);

        self.vel += input_acceleration;
        self.vel /= if self.grounded() {
                ground_friction
            } else {
                air_friction
            };

        self.vel.z -= gravity;

        self.pos += self.vel;
        self.yaw = self.input.yaw;
        self.pitch = self.input.pitch;

        if self.pos.z < character_height {
            self.pos.z = character_height;
            self.vel.z = FixedPoint::zero();
        }
    }

    fn grounded(&self) -> bool {
        let character_height = FixedPoint::fraction(17, 10); // TODO use const in consts instead
        self.pos.z == character_height
    }
}