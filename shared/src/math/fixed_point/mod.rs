mod precalc;

use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::f32::consts::PI as PI32;
use std::f64::consts::PI as PI64;
use std::fmt;

use self::precalc::*;

// fixed-point constants
const FP_PRECISION: u64 = 16;
const FP_RESOLUTION: u64 = 1 << FP_PRECISION;

// trigonometry constants
const FP_SIN_PRECISION_DIFF: u64 = FP_PRECISION - SIN_PRECISION;
const SIN_QUARTER_RESOLUTION: u64 = 1 << (SIN_PRECISION - 2);
const FP_SIN_RESOLUTION_RATIO: u64 = 1 << FP_SIN_PRECISION_DIFF;
const SIN_QUARTER_MASK: u64 = (!0) % SIN_QUARTER_RESOLUTION;
const SIN_INTRA_MASK: u64 = (!0) % FP_SIN_RESOLUTION_RATIO;

custom_derive! {
    #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord,
    NewtypeAdd, NewtypeSub, NewtypeAddAssign, NewtypeSubAssign,
    NewtypeRem, NewtypeNeg)]
    pub struct FixedPoint(i64);
}

// number of bits in the fractional part: FP_PRECISION
// number of bits in the whole part: 64 - 2 * FP_PRECISION
impl FixedPoint {
    pub fn new(value: i64) -> FixedPoint {
        FixedPoint(value << FP_PRECISION)
    }

    pub fn zero() -> FixedPoint {
        FixedPoint(0)
    }

    pub fn one() -> FixedPoint {
        FixedPoint(FP_RESOLUTION as i64)
    }

    pub fn fraction(nominator: i64, denominator: i64) -> Self {
        FixedPoint((nominator << FP_PRECISION) / denominator)
    }

    pub fn abs(&self) -> FixedPoint {
        FixedPoint(self.0.abs())
    }

    pub fn inv_sqrt(self) -> FixedPoint {
        const THREE: i64 = FP_RESOLUTION as i64 * 3;
        if self.0 <= 0 {
            panic!("Attempted to take inverse square root of non-positive number!");
        }
        let mut approx = FP_RESOLUTION as i64;
        for _ in 0..5 { // TODO relate number of iterations to FP_PRECISION
            approx = fp_mul(approx, THREE - fp_mul(fp_mul(self.0, approx), approx)) >> 1;
        }
        FixedPoint(approx)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn is_positive(&self) -> bool {
        self.0 > 0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0
    }

    pub fn mix(self, other: FixedPoint, ratio: FixedPoint) -> FixedPoint {
        self * (Self::one() - ratio) + other * ratio
    }
}

fn fp_mul(a: i64, b: i64) -> i64 {
    (a * b) >> FP_PRECISION
}

fn fp_div(a: i64, b: i64) -> i64 {
    (a << FP_PRECISION) / b
}

impl Mul<FixedPoint> for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(fp_mul(self.0, rhs.0))
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
        self.0 = fp_mul(self.0, rhs.0);
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
        FixedPoint(fp_div(self.0, rhs.0))
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
        self.0 = fp_div(self.0, rhs.0);
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
        let circular = (((self.0).0 % FP_RESOLUTION as i64)
            + FP_RESOLUTION as i64) as u64 % FP_RESOLUTION;
        let intra = circular & SIN_INTRA_MASK;
        let quadrant = circular >> (FP_PRECISION - 2);
        let mut index = ((circular >> FP_SIN_PRECISION_DIFF) & SIN_QUARTER_MASK) as usize;
        let mut next_index = index + 1;
        if quadrant % 2 != 0 {
            index = SIN_QUARTER_RESOLUTION as usize - index;
            next_index = index - 1;
        };
        let sin1 = SIN[index];
        let sin2 = SIN[next_index];
        let mut sin = (sin1 * (FP_SIN_RESOLUTION_RATIO - intra) as i64
            + sin2 * intra as i64) >> FP_SIN_PRECISION_DIFF as i64;
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