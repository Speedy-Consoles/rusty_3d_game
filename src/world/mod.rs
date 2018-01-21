use std::f64::consts::PI;

pub struct World {
    angle: f64,
}

impl World {
    pub fn new() -> Self {
        World {
            angle: 0.0,
        }
    }

    pub fn rotate(&mut self, angle: f64) {
        self.angle = ((self.angle + angle) % (2.0 * PI) + (2.0 * PI)) % (2.0 * PI);
    }

    pub fn get_angle(&self) -> f64 {
        self.angle
    }
}