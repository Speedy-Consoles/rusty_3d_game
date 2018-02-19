use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;
use std::ops::Div;
use std::ops::AddAssign;
use std::ops::SubAssign;
use std::fmt;

use cgmath::Vector3;

use math::fixed_point::FixedPoint;

// TODO rethink this module

#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub z: FixedPoint,
}

impl Vec3 {
    pub fn new(x: FixedPoint, y: FixedPoint, z: FixedPoint) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn zero() -> Vec3 {
        Vec3 { x: 0.into(), y: 0.into(), z: 0.into() }
    }

    pub fn dot(&self, other: Vec3) -> FixedPoint {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length2(&self) -> FixedPoint {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> FixedPoint {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn scale_to(&self, length: FixedPoint) -> Vec3 {
        let my_length = self.length();
        Vec3 {
            x: self.x * length / my_length,
            y: self.y * length / my_length,
            z: self.z * length / my_length,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Self::Output {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Self) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<FixedPoint> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: FixedPoint) -> Vec3 {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Mul<Vec3> for FixedPoint {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
        }
    }
}

impl Div<FixedPoint> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: FixedPoint) -> Vec3 {
        Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, other: Vec3) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, other: Vec3) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}

impl Into<Vector3<f32>> for Vec3 {
    fn into(self) -> Vector3<f32> {
        Vector3::new(
            self.x.into(),
            self.y.into(),
            self.z.into(),
        )
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "v({}, {}, {})", self.x, self.y, self.z)
    }
}

impl fmt::Debug for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}