//! Predefined rules for entropy.

use crate::entropy::{EntropyRules, EntropyNum};
use num_traits::One;
#[cfg(feature = "random")]
use rand::distributions::uniform::SampleUniform;
#[cfg(feature = "random")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "random")]
use rand_pcg::Lcg64Xsh32;
use std::fmt::Debug;

/// A deterministic rule that always returns the lowest value.
#[derive(Debug, Default, Clone, Copy)]
pub struct FixedLow {
}

impl EntropyRules for FixedLow {
    type EntropySeed = ();
    type EntropyModel = ();

    fn generate_model(&self, _seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {}

    /// Always returns `low`.
    fn generate<T: EntropyNum>(
        &self,
        _: &mut Self::EntropyModel,
        low: T,
        _high: T,
    ) -> T
    {
        low
    }
}

/// A deterministic rule that always generates an average result.
#[derive(Debug, Default, Clone, Copy)]
pub struct FixedAverage {
}

impl EntropyRules for FixedAverage {
    type EntropySeed = ();
    type EntropyModel = ();

    fn generate_model(&self, _seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {}

    fn generate<T: EntropyNum>(
        &self,
        _: &mut Self::EntropyModel,
        low: T,
        high: T,
    ) -> T {
        let one: T = One::one();
        (high + low) / (one + one)
    }
}

/// Generate random numbers with uniform distribution.
/// It uses a seedable pseudo random number generator with deterministic output.
///
/// A seed is required to ensure a good level of entropy.
#[cfg(feature = "random")]
#[derive(Debug, Default, Clone, Copy)]
pub struct UniformDistribution {
}

#[cfg(feature = "random")]
impl EntropyRules for UniformDistribution
{
    type EntropySeed = u64;
    type EntropyModel = Lcg64Xsh32;

    fn generate_model(&self, seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {
        Lcg64Xsh32::seed_from_u64(seed.unwrap_or(0))
    }

    fn generate<T: EntropyNum + SampleUniform>(
        &self,
        model: &mut Self::EntropyModel,
        low: T,
        high: T,
    ) -> T {
        model.gen_range(low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_average() {
        let rule = FixedAverage::default();
        assert_eq!(rule.generate(&mut (), 2, 12), 7);
    }

    #[test]
    fn fixed_low() {
        let rule = FixedLow::default();
        assert_eq!(rule.generate(&mut (), 2, 12), 2);
        assert_eq!(rule.generate(&mut (), 4, 3), 4);
    }

    #[cfg(feature = "random")]
    #[test]
    fn uniform_distribution() {
        let seed = 1_204_678_643_940_597_513;
        let rule = UniformDistribution::default();
        for _ in 0..2 {
            let mut model = rule.generate_model(&Some(seed));
            assert_eq!(rule.generate(&mut model, 0, 10), 8);
        }
    }
}
