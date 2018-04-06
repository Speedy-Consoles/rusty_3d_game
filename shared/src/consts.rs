use std::time::Duration;
use std::f64::consts::PI;

use tick_time::TickRate;

// TODO make functions const

// SHARED

// ticks
pub const TICK_SPEED: TickRate = TickRate { per_second: 120 };

// physics
// TODO move const fixed points from model here

// network
pub fn timeout_duration() -> Duration {
    Duration::from_secs(10)
}
pub const MAX_UNACKED_MESSAGES: usize = 1024;
pub fn initial_ack_duration_guess() -> Duration {
    Duration::new(0, 50000000)
}
pub const NEWEST_ACK_DURATION_WEIGHT: f64 = 0.001;
pub const ACK_DURATION_SIGMA_FACTOR: f64 = 3.0;

// CLIENT

pub const BASE_SPEED: TickRate = TickRate { per_second: 60 };

// files
pub const CLIENT_CONFIG_FILE: &'static str = "client_conf.toml";

// graphics
pub const OPTIMAL_SCREEN_RATIO: f64 = 16.0 / 9.0;
pub const Y_FOV: f64 = PI / 3.0;
pub const Z_NEAR: f64 = 0.1;
pub const Z_FAR: f64 = 100.0;
pub const DRAW_SPEED: TickRate = TickRate { per_second: 60 }; // TODO move to config

// network
//pub const MAX_PREDICT_TICKS: usize = 120;
pub const NEWEST_START_TICK_TIME_WEIGHT: f64 = 0.001;
pub const SNAPSHOT_ARRIVAL_SIGMA_FACTOR: f64 = 3.0;

pub const NEWEST_START_PREDICTED_TICK_TIME_WEIGHT: f64 = 0.001;
pub const INPUT_ARRIVAL_SIGMA_FACTOR: f64 = 4.0;

pub fn initial_lag_assumption() -> Duration {
    Duration::new(0, 20000000)
}

// How long to remember when an input was sent.
// The send time is important to determine the predicted tick, once the ack arrives.
// If this time is too short, the estimation of the variance in arrival will be bad,
// because too much of the sample interval will be cut off.
// The longer this time is, the more ticks need to be remembered (TICK_RATE * max_input_keep_time())
// in the worst case (no acks from the server).
pub fn max_input_keep_time() -> Duration {
    Duration::from_secs(10)
}

pub fn connection_request_resend_interval() -> Duration {
    Duration::from_secs(1)
}

pub fn disconnect_force_timeout() -> Duration {
    Duration::from_secs(1)
}

// SERVER