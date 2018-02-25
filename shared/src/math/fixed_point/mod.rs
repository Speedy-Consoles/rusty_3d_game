mod precalc;

use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::f32::consts::PI as PI32;
use std::f64::consts::PI as PI64;
use std::fmt;

const FP_PRECISION: u64 = 16;
const FP_RESOLUTION: i64 = 1 << FP_PRECISION;

custom_derive! {
    #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord,
    NewtypeAdd, NewtypeSub, NewtypeAddAssign, NewtypeSubAssign,
    NewtypeRem, NewtypeNeg)]
    pub struct FixedPoint(i64);
}

impl FixedPoint {
    pub fn new(value: i64) -> FixedPoint {
        FixedPoint(value << FP_PRECISION)
    }

    pub fn zero() -> FixedPoint {
        FixedPoint(0)
    }

    pub fn one() -> FixedPoint {
        FixedPoint(FP_RESOLUTION)
    }

    pub fn fraction(nominator: i64, denominator: i64) -> Self {
        FixedPoint((nominator * FP_RESOLUTION) / denominator)
    }

    pub fn sqrt(self) -> FixedPoint {
        // TODO don't use floats
        let f: f64 = self.into();
        FixedPoint((f.sqrt() * FP_RESOLUTION as f64) as i64)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Mul<FixedPoint> for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(self.0 * rhs.0 / FP_RESOLUTION)
    }
}

impl Mul<i64> for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: i64) -> FixedPoint {
        FixedPoint(self.0 * rhs)
    }
}

impl MulAssign<FixedPoint> for FixedPoint {
    fn mul_assign(&mut self, rhs: FixedPoint) {
        self.0 = self.0 * rhs.0 / FP_RESOLUTION;
    }
}

impl MulAssign<i64> for FixedPoint {
    fn mul_assign(&mut self, rhs: i64) {
        self.0 *= rhs;
    }
}

impl Div<FixedPoint> for FixedPoint {
    type Output = FixedPoint;

    fn div(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint((self.0 * FP_RESOLUTION) / rhs.0)
    }
}

impl Div<i64> for FixedPoint {
    type Output = FixedPoint;

    fn div(self, rhs: i64) -> FixedPoint {
        FixedPoint(self.0 / rhs)
    }
}

impl DivAssign<FixedPoint> for FixedPoint {
    fn div_assign(&mut self, rhs: FixedPoint) {
        self.0 = (self.0 * FP_RESOLUTION) / rhs.0;
    }
}

impl DivAssign<i64> for FixedPoint {
    fn div_assign(&mut self, rhs: i64) {
        self.0 /= rhs;
    }
}

impl From<i64> for FixedPoint {
    fn from(value: i64) -> Self {
        FixedPoint::new(value)
    }
}

impl Into<f64> for FixedPoint {
    fn into(self) -> f64 {
        self.0 as f64 / FP_RESOLUTION as f64
    }
}

impl Into<f32> for FixedPoint {
    fn into(self) -> f32 {
        self.0 as f32 / FP_RESOLUTION as f32
    }
}

impl fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let float: f64 = (*self).into();
        write!(f, "s{:.*}", (0.4 * FP_PRECISION as f64) as usize, float)
    }
}

impl fmt::Debug for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

custom_derive! {
    #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord,
    NewtypeAdd, NewtypeSub, NewtypeMul, NewtypeDiv, NewtypeAddAssign, NewtypeSubAssign,
    NewtypeRem, NewtypeNeg, NewtypeFrom)]
    pub struct FPAngle(FixedPoint);
}

impl FPAngle {
    pub fn fraction(nominator: i64, denominator: i64) -> FPAngle {
        FPAngle(FixedPoint::fraction(nominator, denominator))
    }

    pub fn whole() -> FPAngle {
        FPAngle(FixedPoint::new(1))
    }

    pub fn half() -> FPAngle {
        FPAngle(FixedPoint::fraction(1, 2))
    }

    pub fn quarter() -> FPAngle {
        FPAngle(FixedPoint::fraction(1, 4))
    }

    pub fn zero() -> FPAngle {
        FPAngle(FixedPoint::new(0))
    }

    pub fn sin(&self) -> FixedPoint {
        const RESOLUTION_RATIO: i64 = FP_RESOLUTION / precalc::SIN_RESOLUTION;
        let circular = (((self.0).0 % FP_RESOLUTION) + FP_RESOLUTION) % FP_RESOLUTION;
        let full_index = circular / RESOLUTION_RATIO;
        let intra = circular % RESOLUTION_RATIO;
        let quadrant = full_index / precalc::SIN_QUARTER_RESOLUTION;
        let mut index = full_index as usize % precalc::SIN_QUARTER_RESOLUTION as usize;
        let mut next_index = index + 1;
        if quadrant % 2 != 0 {
            index = precalc::SIN_QUARTER_RESOLUTION as usize - index;
            next_index = index - 1;
        };
        let sin1 = precalc::SIN[index];
        let sin2 = precalc::SIN[next_index];
        let mut sin = (sin1 * (RESOLUTION_RATIO - intra) + sin2 * intra) / RESOLUTION_RATIO;
        if quadrant / 2 != 0 {
            sin = -sin
        };
        FixedPoint(sin)
    }

    pub fn cos(&self) -> FixedPoint {
        (*self + FPAngle::quarter()).sin()
    }

    pub fn from_tau_float(float: f64) -> FPAngle {
        FPAngle(FixedPoint((float * FP_RESOLUTION as f64) as i64))
    }

    pub fn rad_f32(self) -> f32 {
        let f: f32 = self.0.into();
        f * 2.0 * PI32
    }

    pub fn rad_f64(self) -> f64 {
        let f: f64 = self.0.into();
        f * 2.0 * PI64
    }
}

impl fmt::Display for FPAngle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let float: f64 = self.0.into();
        write!(f, "a{:.*}", (0.3 * FP_PRECISION as f64) as usize, float)
    }
}

impl fmt::Debug for FPAngle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}