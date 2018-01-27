use std::time::Duration;
use std::f64::consts::PI;

// shared
pub const TICK_SPEED: u32 = 60;
pub fn tick_length() -> Duration {
    Duration::from_secs(1) / TICK_SPEED
}

// client
pub const OPTIMAL_SCREEN_RATIO: f64 = 16.0 / 9.0;
pub const Y_FOV: f64 = PI / 3.0;
pub const Z_NEAR: f64 = 0.1;
pub const Z_FAR: f64 = 100.0;