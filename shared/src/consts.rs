use std::time::Duration;
use std::f64::consts::PI;

// TODO make function calls const variables

// SHARED

// ticks
pub const TICK_SPEED: u64 = 120;
pub fn tick_duration() -> Duration { // TODO get rid of this
    Duration::from_secs(1) / TICK_SPEED as u32
}

// physics
// TODO move const fixed points from model here

// network
pub fn time_out_delay() -> Duration {
    Duration::from_secs(30)
}

// CLIENT

// files
pub const CLIENT_CONFIG_FILE: &'static str = "client_conf.toml";

// graphics
pub const OPTIMAL_SCREEN_RATIO: f64 = 16.0 / 9.0;
pub const Y_FOV: f64 = PI / 3.0;
pub const Z_NEAR: f64 = 0.1;
pub const Z_FAR: f64 = 100.0;
pub const DRAW_SPEED: u64 = 60; // TODO move to config
pub fn draw_interval() -> Duration { // TODO get rid of this
    Duration::from_secs(1) / DRAW_SPEED as u32
}

// network
//pub const MAX_PREDICT_TICKS: usize = 120;
pub const NEWEST_START_TICK_TIME_WEIGHT: f64 = 0.001;
pub const NEWEST_START_TICK_TIME_DEVIATION_WEIGHT: f64 = 0.005;
pub const SNAPSHOT_ARRIVAL_SIGMA_FACTOR: f64 = 3.0;