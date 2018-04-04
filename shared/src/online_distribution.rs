use std::ops::Sub;
use std::ops::Add;
use std::time::Duration;

use util;
use util::Mix;

pub struct OnlineDistribution<T>
where T: Copy + PartialOrd + Mix + Sub<T, Output=Duration> + Add<Duration, Output=T>
{
    mean: T,
    variance: f64,
}

// TODO the online distribution does not calculate the distribution from CrapNet

impl<T> OnlineDistribution<T>
where T: Copy + PartialOrd + Mix + Sub<T, Output=Duration> + Add<Duration, Output=T>
{
    pub fn new(sample: T) -> OnlineDistribution<T> {
        OnlineDistribution {
            mean: sample,
            variance: 0.0,
        }
    }

    pub fn add_sample(&mut self, sample: T, weight: f64) {
        let old_diff = if sample > self.mean {
            util::duration_as_float(sample - self.mean)
        } else {
            -util::duration_as_float(self.mean - sample)
        };
        self.mean = self.mean.mix(&sample, weight);
        let new_diff = if sample > self.mean {
            util::duration_as_float(sample - self.mean)
        } else {
            -util::duration_as_float(self.mean - sample)
        };
        self.variance = self.variance.mix(&(old_diff * new_diff), weight);
    }

    pub fn mean(&self) -> T {
        self.mean
    }

    pub fn sigma_dev(&self, sigma_factor: f64) -> Duration {
        util::duration_from_float(self.variance.sqrt() * sigma_factor)
    }
}