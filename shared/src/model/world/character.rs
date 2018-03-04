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
        let max_expansion = FixedPoint::new(0);
        let min_expansion = FixedPoint::fraction(-8, 10);
        let ground_acceleration = FixedPoint::fraction(1, 80);
        let max_walking_speed = FixedPoint::fraction(1, 20);
        let ground_friction = FixedPoint::one() + ground_acceleration / max_walking_speed;
        let air_acceleration = FixedPoint::fraction(1, 2400);
        let air_friction = FixedPoint::fraction(100, 99);
        let max_expansion_acceleration = FixedPoint::fraction(1, 100);
        let jump_velocity = FixedPoint::fraction(1, 30);
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
        if self.grounded() && self.input.jumping {
            self.jumping = true;
        }

        // calculate acceleration
        let input_expansion_acceleration = if self.jumping || !self.input.crouch {
            max_expansion_acceleration
        } else {
            -max_expansion_acceleration
        };
        if !input_acceleration.is_zero() {
            input_acceleration = if self.grounded() {
                input_acceleration.scale_to(ground_acceleration)
            } else {
                input_acceleration.scale_to(air_acceleration)
            };
        }

        // apply acceleration
        if self.expansion == max_expansion && self.grounded() && self.jumping {
            self.vel.z += jump_velocity;
            self.jumping = false;
        }
        self.vel += input_acceleration;
        self.vel.z -= gravity;
        if (self.expansion_speed * input_expansion_acceleration).is_negative() {
            self.expansion_speed = FixedPoint::zero();
        }
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

        // cap expansion
        let predicted_expansion = self.expansion + self.expansion_speed;
        if predicted_expansion < min_expansion {
            self.expansion_speed = min_expansion - self.expansion
        } else if predicted_expansion > max_expansion {
            self.expansion_speed = max_expansion - self.expansion
        }

        // apply velocity
        let mut exp_speed = Vec3::zero();
        if self.grounded() {
            exp_speed.z += self.expansion_speed;
        } else {
            exp_speed.z += self.expansion_speed / 4;
        }
        self.expansion += self.expansion_speed;
        self.pos += self.vel + exp_speed;

        // cap position
        if self.pos.z < self.current_height() {
            self.pos.z = self.current_height();
            self.vel.z = FixedPoint::zero();
        }

        // apply view dir
        self.view_dir = self.input.view_dir;
    }

    fn grounded(&self) -> bool {
        (self.pos.z - self.current_height()).is_zero()
    }

    fn current_height(&self) -> FixedPoint {
        let character_height = FixedPoint::fraction(17, 10); // TODO use const in consts instead
        character_height + self.expansion
    }
}