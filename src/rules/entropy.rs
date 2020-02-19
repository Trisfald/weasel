//! Predefined rules for entropy.

use crate::entropy::EntropyRules;
use num_traits::{Num, One};
#[cfg(feature = "random")]
use rand::distributions::uniform::SampleUniform;
#[cfg(feature = "random")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "random")]
use rand_pcg::Lcg64Xsh32;
use std::fmt::Debug;
use std::marker::PhantomData;

/// A deterministic rule that always returns the lowest value.
#[derive(Debug, Default, Clone, Copy)]
pub struct FixedLow<T> {
    _phantom: PhantomData<T>,
}

impl<T: PartialOrd + Copy + Num + Debug> EntropyRules for FixedLow<T> {
    type EntropySeed = ();
    type EntropyModel = ();
    type EntropyOutput = T;

    fn generate_model(&self, _seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {}

    /// Always returns `low`.
    fn generate(
        &self,
        _: &mut Self::EntropyModel,
        low: Self::EntropyOutput,
        _high: Self::EntropyOutput,
    ) -> Self::EntropyOutput {
        low
    }
}

/// A deterministic rule that always generates an average result.
#[derive(Debug, Default, Clone, Copy)]
pub struct FixedAverage<T> {
    _phantom: PhantomData<T>,
}

impl<T: PartialOrd + Copy + Num + Debug> EntropyRules for FixedAverage<T> {
    type EntropySeed = ();
    type EntropyModel = ();
    type EntropyOutput = T;

    fn generate_model(&self, _seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {}

    fn generate(
        &self,
        _: &mut Self::EntropyModel,
        low: Self::EntropyOutput,
        high: Self::EntropyOutput,
    ) -> T {
        let one: Self::EntropyOutput = One::one();
        (high + low) / (one + one)
    }
}

/// Generate random numbers with uniform distribution.
/// It uses a seedable pseudo random number generator with deterministic output.
///
/// A seed is required to ensure a good level of entropy.
#[cfg(feature = "random")]
#[derive(Debug, Default, Clone, Copy)]
pub struct UniformDistribution<T> {
    _phantom: PhantomData<T>,
}

#[cfg(feature = "random")]
impl<T> EntropyRules for UniformDistribution<T>
where
    T: PartialOrd + Copy + Num + Debug + SampleUniform,
{
    type EntropySeed = u64;
    type EntropyModel = Lcg64Xsh32;
    type EntropyOutput = T;

    fn generate_model(&self, seed: &Option<Self::EntropySeed>) -> Self::EntropyModel {
        Lcg64Xsh32::seed_from_u64(seed.unwrap_or(0))
    }

    fn generate(
        &self,
        model: &mut Self::EntropyModel,
        low: Self::EntropyOutput,
        high: Self::EntropyOutput,
    ) -> Self::EntropyOutput {
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
