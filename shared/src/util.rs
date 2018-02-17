use std::time::Duration;

pub fn mult_duration(duration: &Duration, factor: u64) -> Duration {
    let secs = duration.as_secs() * factor;
    let nanos = duration.subsec_nanos() as u64 * factor;
    let new_secs = nanos / 1_000_000_000;
    Duration::new(
        secs + new_secs,
        (nanos - new_secs * 1_000_000_000) as u32,
    )
}

pub fn elapsed_ticks(duration: &Duration, tick_speed: u32) -> u64 {
    let sec_diff = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    (sec_diff * tick_speed as f64).floor() as u64
}