use std::ops::Mul;
use std::ops::Div;
use std::f32::consts::PI as PI32;
use std::f64::consts::PI as PI64;
use std::fmt;

const FP_PRECISION: u64 = 16;
const FP_FACTOR: f64 = (1u64 << FP_PRECISION) as f64;

custom_derive! {
    #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord,
    NewtypeAdd, NewtypeSub, NewtypeAddAssign, NewtypeSubAssign,
    NewtypeRem, NewtypeNeg)]
    pub struct FixedPoint(i64);
}

impl FixedPoint {
    pub fn new(value: i64) -> FixedPoint {
        FixedPoint::fraction(value, 1)
    }

    pub fn fraction(nominator: i64, denominator: i64) -> Self {
        FixedPoint((nominator << FP_PRECISION) / denominator)
    }

    pub fn sqrt(self) -> FixedPoint {
        // TODO don't use floats
        let f: f64 = self.into();
        FixedPoint((f.sqrt() * FP_FACTOR) as i64)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Mul<FixedPoint> for FixedPoint {
    type Output = FixedPoint;

    fn mul(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(self.0 * rhs.0 >> FP_PRECISION)
    }
}

impl Div<FixedPoint> for FixedPoint {
    type Output = FixedPoint;

    fn div(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint((self.0 << FP_PRECISION) / rhs.0)
    }
}

impl From<i64> for FixedPoint {
    fn from(number: i64) -> Self {
        FixedPoint(number << FP_PRECISION)
    }
}

impl Into<f64> for FixedPoint {
    fn into(self) -> f64 {
        self.0 as f64 / FP_FACTOR
    }
}

impl Into<f32> for FixedPoint {
    fn into(self) -> f32 {
        self.0 as f32 / FP_FACTOR as f32
    }
}

impl fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let float: f64 = (*self).into();
        write!(f, "s{:.*}", (0.3 * FP_PRECISION as f64) as usize, float)
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
        // TODO don't use floats
        let f: f64 = self.0.into();
        FixedPoint(((f * 2.0 * PI64).sin() * FP_FACTOR) as i64)
    }

    pub fn cos(&self) -> FixedPoint {
        // TODO don't use floats
        let f: f64 = self.0.into();
        FixedPoint(((f * 2.0 * PI64).cos() * FP_FACTOR) as i64)
    }

    pub fn from_tau_float(float: f64) -> FPAngle {
        FPAngle(FixedPoint((float * FP_FACTOR) as i64))
    }

    pub fn rad(self) -> f32 {
        let f: f32 = self.0.into();
        f * 2.0 * PI32
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