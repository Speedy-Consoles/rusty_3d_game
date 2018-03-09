use std::time::Duration;
use std::time::Instant;

pub fn duration_as_float(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9
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
    let dur = duration_as_float(duration);
    let new_dur = dur * factor;
    let secs = new_dur as u64;
    let nanos = ((new_dur - secs as f64) * 1e9) as u32;
    Duration::new(secs, nanos)
}

pub fn mix_time(a: Instant, b: Instant, factor: f64) -> Instant {
    if a > b {
        return mix_time(b, a, 1.0 - factor);
    }
    a + mult_duration_float(b - a, factor)
}

pub fn elapsed_ticks_float(duration: Duration, tick_speed: u32) -> f64 {
    let sec_diff = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    sec_diff * tick_speed as f64
}

pub fn elapsed_ticks(duration: Duration, tick_speed: u32) -> u64 {
    elapsed_ticks_float(duration, tick_speed) as u64
}

pub fn intra_tick(tick_time: Instant, next_tick_time: Instant) -> f64 {
    let part_dur = Instant::now() - tick_time;
    let whole_dur = next_tick_time - tick_time;
    duration_as_float(part_dur) / duration_as_float(whole_dur)
}