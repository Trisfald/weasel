//! Generic implementations for all purpose statuses.

use crate::status::StatusDuration;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// A simple generic status.
#[derive(PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SimpleStatus<I, V> {
    id: I,
    effect: V,
    max_duration: Option<StatusDuration>,
}

impl<I, V: Copy> SimpleStatus<I, V> {
    /// Creates a new `SimpleStatus`.
    pub fn new(id: I, effect: V, max_duration: Option<StatusDuration>) -> SimpleStatus<I, V> {
        SimpleStatus {
            id,
            effect,
            max_duration,
        }
    }

    /// Returns the effect provoked by this status.
    pub fn effect(&self) -> V {
        self.effect
    }

    /// Change the effect of this status.
    pub fn set_effect(&mut self, effect: V) {
        self.effect = effect;
    }

    /// Returns the maximum duration of this status.
    /// `None` means infinite duration.
    pub fn max_duration(&self) -> Option<StatusDuration> {
        self.max_duration
    }
}

#[cfg(not(feature = "serialization"))]
impl<I, V> Id for SimpleStatus<I, V>
where
    I: Debug + Hash + Eq + Clone,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[cfg(feature = "serialization")]
impl<I, V> Id for SimpleStatus<I, V>
where
    I: Debug + Hash + Eq + Clone + Serialize + for<'a> Deserialize<'a>,
{
    type Id = I;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
