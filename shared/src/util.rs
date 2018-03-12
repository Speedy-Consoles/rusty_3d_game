use std::time::Duration;
use std::time::Instant;

use cgmath::Vector3;

use consts::TICK_SPEED;

pub fn duration_as_float(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9
}

pub fn duration_from_float(duration_float: f64) -> Duration {
    let secs = duration_float as u64;
    let nanos = ((duration_float - secs as f64) * 1e9) as u32;
    Duration::new(secs, nanos)
}

pub fn mult_duration(duration: Duration, factor: u64) -> Duration {
    let secs = duration.as_secs() * factor;
    let nanos = duration.subsec_nanos() as u64 * factor;
    let new_secs = nanos / 1_000_000_000;
    Duration::new(
        secs + new_secs,
        (nanos - new_secs * 1_000_000_000) as u32,
    )
}

pub fn mult_duration_float(duration: Duration, factor: f64) -> Duration {
    assert!(factor >= 0.0);
    duration_from_float(duration_as_float(duration) * factor)
}

pub fn elapsed_events(duration: Duration, event_rate: u64) -> u64 {
    let ticks_sec = duration.as_secs() * event_rate;
    let ticks_nano = duration.subsec_nanos() as u64 * TICK_SPEED / 1_000_000_000;
    ticks_sec + ticks_nano
}

pub trait Mix {
    fn mix(&self, other: &Self, ratio: f64) -> Self;
}

impl Mix for f32 {
    fn mix(&self, other: &Self, ratio: f64) -> Self {
        *self * (1.0 - ratio) as f32 + *other * ratio as f32
    }
}

impl Mix for f64 {
    fn mix(&self, other: &Self, ratio: f64) -> Self {
        *self * (1.0 - ratio) + *other * ratio
    }
}

impl Mix for Vector3<f32> {
    fn mix(&self, other: &Self, ratio: f64) -> Self {
        Vector3::new(
            self.x.mix(&other.x, ratio),
            self.y.mix(&other.y, ratio),
            self.z.mix(&other.z, ratio)
        )
    }
}

impl Mix for Instant {
    fn mix(&self, other: &Instant, factor: f64) -> Instant {
        if *self > *other {
            return other.mix(self, 1.0 - factor);
        }
        *self + mult_duration_float(*other - *self, factor)
    }
}