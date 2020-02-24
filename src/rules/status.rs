//! Generic implementations for all purpose statuses.

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
    duration: Option<u16>, 
}

impl<I, V: Copy> SimpleStatus<I, V> {
    /// Creates a new `SimpleStatus`.
    pub fn new(id: I, effect: V, duration: Option<u16>) -> SimpleStatus<I, V> {
        SimpleStatus { id, effect, duration }
    }

    /// Returns the effect provoked by this status.
    pub fn effect(&self) -> V {
        self.effect
    }

    /// Change the effect of this status.
    pub fn set_effect(&mut self, effect: V) {
        self.effect = effect;
    }

    /// Returns the remaining duration of this status.
    /// `None` means infinite duration.
    pub fn duration(&self) -> Option<u16> {
        self.duration
    }

    /// Decreases the duration by one.
    pub fn advance(&mut self) {
        if let Some(duration) = self.duration {
            if duration > 0 {
                self.duration = Some(duration - 1);
            }
        }
    }

    /// Returns `true` if this status has ended.
    pub fn finished(&self) -> bool {
        if let Some(duration) = self.duration {
            duration == 0
        } else {
            false
        }
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
