use std::time::Duration;
use std::f64::consts::PI;

//use math::FixedPoint; // TODO

// SHARED

// ticks
pub const TICK_SPEED: u32 = 60;
pub fn tick_interval() -> Duration { // TODO make const
    Duration::from_secs(1) / TICK_SPEED
}

// physics
//pub const WALKING_SPEED: FixedPoint = FixedPoint::fraction(1, 100); // TODO


// CLIENT

// files
pub const CLIENT_CONFIG_FILE: &'static str = "client_conf.toml";

// graphics
pub const OPTIMAL_SCREEN_RATIO: f64 = 16.0 / 9.0;
pub const Y_FOV: f64 = PI / 3.0;
pub const Z_NEAR: f64 = 0.1;
pub const Z_FAR: f64 = 100.0;
pub const DRAW_SPEED: u32 = 60; // TODO move to config
pub fn draw_interval() -> Duration { // TODO move to config, make const
    Duration::from_secs(1) / DRAW_SPEED
}

// prediction
pub const MAX_PREDICT_TICKS: usize = 120;