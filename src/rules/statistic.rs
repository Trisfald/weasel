//! Generic implementations for different types of statistic.

use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::cmp::PartialOrd;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Add;

/// A simple generic statistic storing current value, minimum and maximum value.
#[derive(PartialEq, Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SimpleStatistic<I, V> {
    id: I,
    min: V,
    max: V,
    value: V,
}

impl<I, V: Copy + Default> SimpleStatistic<I, V> {
    /// Creates a new `SimpleStatistic` with `value` equal to `max`
    /// and `min` equal to `V::default()`.
    pub fn new(id: I, max: V) -> SimpleStatistic<I, V> {
        SimpleStatistic::with_value(id, V::default(), max, max)
    }

    /// Creates a new `SimpleStatistic` with the given value.
    pub fn with_value(id: I, min: V, max: V, value: V) -> SimpleStatistic<I, V> {
        SimpleStatistic {
            id,
            min,
            max,
            value,
        }
    }
}

impl<I, V> SimpleStatistic<I, V>
where
    V: Copy + PartialOrd + Add<Output = V>,
{
    /// Returns the current value of this statistic.
    pub fn value(&self) -> V {
        self.value
    }

    /// Returns the minimum value of this statistic.
    pub fn min(&self) -> V {
        self.min
    }

    /// Returns the maximum value of this statistic.
    pub fn max(&self) -> V {
        self.max
    }

    /// Sets the current value to the new one, respecting the min/max bounds.
    pub fn set_value(&mut self, value: V) {
        self.value = value;
        if self.value < self.min {
            self.value = self.min;
        } else if self.value > self.max {
            self.value = self.max;
        }
    }

    /// Adds an increment `inc` to the value, respecting the min/max bounds.
    pub fn add(&mut self, inc: V) {
        self.set_value(self.value + inc);
    }
}

#[cfg(not(feature = "serialization"))]
impl<I, V> Id for SimpleStatistic<I, V>
where
    I: Debug + Hash + Eq + Clone,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[cfg(feature = "serialization")]
impl<I, V> Id for SimpleStatistic<I, V>
where
    I: Debug + Hash + Eq + Clone + Serialize + for<'a> Deserialize<'a>,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_statistic_bounds() {
        let mut stat = SimpleStatistic::with_value(1, 10, 20, 15);
        stat.add(100);
        assert_eq!(stat.value(), stat.max());
        stat.add(-100);
        assert_eq!(stat.value(), stat.min());
    }
}
