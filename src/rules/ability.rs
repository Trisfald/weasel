//! Generic implementations for all purpose abilities.

use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// A simple generic ability.
#[derive(PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SimpleAbility<I, V> {
    id: I,
    power: V,
}

impl<I: Send, V: Copy> SimpleAbility<I, V> {
    /// Creates a new `SimpleAbility`.
    pub fn new(id: I, power: V) -> SimpleAbility<I, V> {
        SimpleAbility { id, power }
    }

    /// Returns this ability's power.
    pub fn power(&self) -> V {
        self.power
    }

    /// Change the power of this ability.
    pub fn set_power(&mut self, power: V) {
        self.power = power;
    }
}

#[cfg(not(feature = "serialization"))]
impl<I, V> Id for SimpleAbility<I, V>
where
    I: Debug + Hash + Eq + Clone + Send,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[cfg(feature = "serialization")]
impl<I, V> Id for SimpleAbility<I, V>
where
    I: Debug + Hash + Eq + Clone + Send + Serialize + for<'a> Deserialize<'a>,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
