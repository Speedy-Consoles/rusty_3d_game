use std::time::Instant;
use std::time::Duration;
use std::ops::Mul;
use std::ops::Div;

#[derive(Debug, Clone, Copy)]
pub struct TickInstant {
    pub tick: u64,
    pub intra_tick: f64,
}

impl TickInstant {
    pub fn new(start_tick_time: Instant, now: Instant, rate: TickRate) -> TickInstant {
        let tick_diff = (now - start_tick_time) * rate;
        TickInstant {
            tick: tick_diff.ticks,
            intra_tick: tick_diff.tick_fraction,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TickDiff {
    pub ticks: u64,
    pub tick_fraction: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct TickRate {
    pub per_second: u64, // TODO make private after https://github.com/rust-lang/rust/issues/24111
}

impl TickRate {
    pub fn from_per_second(per_second: u64) -> TickRate {
        TickRate { per_second }
    }

    pub fn per_second(&self) -> u64 {
        self.per_second
    }
}

impl Mul<Duration> for TickRate {
    type Output = TickDiff;

    fn mul(self, rhs: Duration) -> TickDiff {
        let ticks_sec = rhs.as_secs() * self.per_second;
        let nano_prod = rhs.subsec_nanos() as u64 * self.per_second;
        let ticks_nano = nano_prod / 1_000_000_000;
        let sub_ticks = (nano_prod % 1_000_000_000) as f64 * 1e-9;
        TickDiff {
            ticks: ticks_sec + ticks_nano,
            tick_fraction: sub_ticks,
        }
    }
}

impl Mul<TickRate> for Duration {
    type Output = TickDiff;

    fn mul(self, rhs: TickRate) -> TickDiff {
        rhs * self
    }
}

impl Div<TickRate> for TickDiff {
    type Output = Duration;

    fn div(self, rhs: TickRate) -> Duration {
        let whole_secs = self.ticks / rhs.per_second;
        let nano_secs = (
                (self.ticks % rhs.per_second) * 1_000_000_000
                + (self.tick_fraction * 1e9) as u64
            ) / rhs.per_second;
        Duration::new(whole_secs, nano_secs as u32)
    }
}

impl Div<TickRate> for u64 {
    type Output = Duration;

    fn div(self, rhs: TickRate) -> Duration {
        let whole_secs = self / rhs.per_second;
        let nano_secs = (self % rhs.per_second) * 1_000_000_000 / rhs.per_second;
        Duration::new(whole_secs, nano_secs as u32)
    }
}

impl Div<TickRate> for f64 {
    type Output = Duration;

    fn div(self, rhs: TickRate) -> Duration {
        let seconds = self / rhs.per_second as f64;
        let whole_secs = seconds as u64;
        let nano_secs = (seconds - whole_secs as f64) * 1e9;
        Duration::new(whole_secs, nano_secs as u32)
    }
}