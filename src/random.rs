extern crate num_traits;

use num_traits::{Num, One};

pub trait RandomnessRule {
    /// Generates a random value within half-open range [low, high)
    fn generate<T: PartialOrd + Copy + Num>(&mut self, low: T, high: T) -> T;
}

/// A non random rule that always generates an average result
#[derive(Debug, Default)]
pub struct FixedAverage {}

impl RandomnessRule for FixedAverage {
    /// Generates a random value
    /// 
    /// # Panics
    /// 
    /// The function panics if low is greater than high
    fn generate<T: PartialOrd + Copy + Num>(&mut self, low: T, high: T) -> T {
        if low > high {
            panic!("low can't be greater than high!");
        }
        let one: T = One::one();
        (high + low) / (one + one)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_average() {
        let mut rule = FixedAverage::default();
        assert_eq!(rule.generate(2, 12), 7);
    }

    #[test]
    #[should_panic]
    fn fixed_average_panic() {
        FixedAverage::default().generate(3, 1);
    }
}