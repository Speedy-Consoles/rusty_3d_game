use std::time::Instant;
use std::time::Duration;

use shared::consts::TICK_SPEED;

#[derive(Debug, Clone, Copy)]
pub struct TickInstant {
    pub tick: u64,
    pub intra_tick: f64,
}

impl TickInstant {
    pub fn new(start_tick_time: Instant, now: Instant) -> TickInstant {
        let tick_diff = TickDiff::elapsed_ticks(now - start_tick_time);
        TickInstant {
            tick: tick_diff.ticks,
            intra_tick: tick_diff.sub_ticks,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TickDiff {
    pub ticks: u64,
    pub sub_ticks: f64,
}

impl TickDiff {
    pub fn elapsed_ticks(duration: Duration) -> TickDiff {
        let ticks_sec = duration.as_secs() * TICK_SPEED;
        let nano_prod = duration.subsec_nanos() as u64 * TICK_SPEED;
        let ticks_nano = nano_prod / 1_000_000_000;
        let sub_ticks = (nano_prod % 1_000_000_000) as f64 * 1e-9;
        TickDiff {
            ticks: ticks_sec + ticks_nano,
            sub_ticks,
        }
    }
}