use math::FixedPoint;
use math::FPAngle;
use math::Vec3;

#[derive(Default, Copy, Clone)]
pub struct ViewDir {
    yaw: FPAngle,
    pitch: FPAngle,
}

impl ViewDir {
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
}

#[derive(Default, Copy, Clone)]
pub struct CharacterInput {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    pub crouch: bool,
    pub jumping: bool, // TODO consider to make this a counter, so we don't need to reset it
    pub view_dir: ViewDir,
}

impl CharacterInput {
    pub fn reset_flags(&mut self) {
        self.jumping = false;
    }
}

#[derive(Clone)]
pub struct Character {
    input: CharacterInput,
    pos: Vec3,
    vel: Vec3,
    view_dir: ViewDir,
    jumping: bool,
    expansion: FixedPoint,
    expansion_speed: FixedPoint,
}

impl Character {
    pub fn new() -> Character {
        let character_height = FixedPoint::fraction(17, 10); // TODO use const in consts instead
        Character {
            input: Default::default(),
            pos: Vec3::new(
                FixedPoint::zero(),
                FixedPoint::zero(),
                character_height,
            ),
            vel: Vec3::zero(),
            view_dir: Default::default(),
            jumping: false,
            expansion: FixedPoint::zero(),
            expansion_speed: FixedPoint::zero(),
        }
    }

    pub fn set_input(&mut self, input: CharacterInput) {
        self.input = input;
    }

    pub fn get_pos(&self) -> Vec3 {
        self.pos
    }

    pub fn get_view_dir(&self) -> ViewDir {
        self.view_dir
    }

    pub fn tick(&mut self) {
        // TODO move these to consts
        let max_expansion = FixedPoint::fraction(2, 10);
        let min_expansion = FixedPoint::fraction(-8, 10);
        let ground_acceleration = FixedPoint::fraction(1, 80);
        let max_walking_speed = FixedPoint::fraction(1, 20);
        let ground_friction = FixedPoint::one() + ground_acceleration / max_walking_speed;
        let air_acceleration = FixedPoint::fraction(1, 2400);
        let air_friction = FixedPoint::fraction(100, 99);
        let max_expansion_acceleration = FixedPoint::fraction(1, 150);
        let expansion_friction = FixedPoint::fraction(10, 8);
        let gravity = FixedPoint::fraction(1, 1440);

        // calculate move direction
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
        let ys = self.view_dir.yaw.sin();
        let yc = self.view_dir.yaw.cos();
        input_acceleration = Vec3::new(
            input_acceleration.x * yc - input_acceleration.y * ys,
            input_acceleration.x * ys + input_acceleration.y * yc,
            input_acceleration.z
        );
        if !self.grounded() || self.expansion == max_expansion {
            self.jumping = false;
        } else if self.input.jumping {
            self.jumping = true;
        }
        let mut input_expansion_acceleration = if self.jumping {
            max_expansion_acceleration
        } else if self.input.crouch {
            -max_expansion_acceleration
        } else {
            -self.expansion / 35 - self.expansion_speed / 2
        };

        // calculate acceleration
        if !input_acceleration.is_zero() {
            input_acceleration = if self.grounded() {
                input_acceleration.scale_to(ground_acceleration)
            } else {
                input_acceleration.scale_to(air_acceleration)
            };
        }
        if input_expansion_acceleration > max_expansion_acceleration {
            input_expansion_acceleration = max_expansion_acceleration;
        } else if input_expansion_acceleration < -max_expansion_acceleration {
            input_expansion_acceleration = -max_expansion_acceleration;
        }

        // apply acceleration
        self.vel += input_acceleration;
        self.vel.z -= gravity;
        self.expansion_speed += input_expansion_acceleration;

        // apply friction
        if self.grounded() {
            self.vel.x /= ground_friction;
            self.vel.y /= ground_friction;
        } else {
            self.vel.x /= air_friction;
            self.vel.y /= air_friction;
        };
        self.vel.z /= air_friction;
        self.expansion_speed /= expansion_friction;

        // apply velocity
        self.pos += self.vel;
        self.expansion += self.expansion_speed;
        self.view_dir = self.input.view_dir;

        // cap position
        let saved_exp_speed = self.expansion_speed;
        if self.expansion < min_expansion {
            self.expansion = min_expansion;
            self.expansion_speed = FixedPoint::zero();
        } else if self.expansion > max_expansion {
            self.expansion = max_expansion;
            self.expansion_speed = FixedPoint::zero();
        }
        if self.pos.z < self.current_height() {
            self.pos.z = self.current_height();
            self.vel.z = saved_exp_speed;
        }
    }

    fn grounded(&self) -> bool {
        (self.pos.z - self.current_height()).is_zero()
    }

    fn current_height(&self) -> FixedPoint {
        let character_height = FixedPoint::fraction(17, 10); // TODO use const in consts instead
        character_height + self.expansion
    }
}