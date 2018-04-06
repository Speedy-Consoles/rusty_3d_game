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

#[cfg(test)]
mod test {

    use rand;
    use rand::distributions::IndependentSample;
    use rand::distributions::Gamma;
    use rand::Rng;
    use rand::SeedableRng;
    use rand::StdRng;

    use util;

    use super::OnlineDistribution;

    #[test]
    fn test() {
        let seed: &[_] = &[42];
        let mut rng: StdRng = SeedableRng::from_seed(seed);

        let mean = 10.0;
        let std_dev = 20.0;

        let shape = mean * mean / (std_dev * std_dev);
        let scale = std_dev * std_dev / mean;
        let distribution = Gamma::new(shape, scale);

        let initial_sample = distribution.ind_sample(&mut rng);
        let mut od = OnlineDistribution::new(util::duration_from_float(initial_sample));

        for _ in 0..100000 {
            let sample = distribution.ind_sample(&mut rng);
            od.add_sample(util::duration_from_float(sample), 0.001);
        }

        let calculated_std_dev = util::duration_as_float(od.sigma_dev(1.0));

        println!("{:?}", calculated_std_dev);
        assert!((std_dev - calculated_std_dev).abs() / std_dev < 0.1);
    }
}