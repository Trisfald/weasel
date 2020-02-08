//! User defined extension for battle rules functionalities.

use crate::battle::BattleRules;
#[cfg(feature = "serialization")]
use crate::error::{WeaselError, WeaselResult};
#[cfg(feature = "serialization")]
use crate::event::Event;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// Numerical identifier to distinguish user events.
pub type UserEventId = u16;

/// Rules to extend some aspects of the battle with user defined behavior.
pub trait UserRules<R: BattleRules> {
    /// See [UserMetricId](type.UserMetricId.html).
    type UserMetricId: Eq + Hash + Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [UserEventPackage](type.UserEventPackage.html).
    type UserEventPackage: UserEventPacker<R>;
}

/// Id of user defined metrics.
pub type UserMetricId<R> = <<R as BattleRules>::UR as UserRules<R>>::UserMetricId;

#[cfg(feature = "serialization")]
/// Type containing the data to serialize and deserialize all defined user events.\
/// Use `()` if you didn't define any user event.
pub type UserEventPackage<R> = <<R as BattleRules>::UR as UserRules<R>>::UserEventPackage;

#[cfg(feature = "serialization")]
/// Stores one user event payload and manages its serialization/deserialization.
pub trait UserEventPacker<R>: Serialize + for<'a> Deserialize<'a>
where
    R: BattleRules,
{
    /// Returns a boxed trait object version of this packed user event.
    ///
    /// Returns an error if the conversion failed.
    fn boxed(self) -> WeaselResult<Box<dyn Event<R>>, R>;

    /// Returns a UserEventPacker corresponding to the user event contained inside `event`.
    ///
    /// Fails if `event` is not an user event or if the conversion failed.
    fn flattened(event: Box<dyn Event<R>>) -> WeaselResult<Self, R>;
}

#[cfg(feature = "serialization")]
impl<R> UserEventPacker<R> for ()
where
    R: BattleRules,
{
    fn boxed(self) -> WeaselResult<Box<dyn Event<R>>, R> {
        Err(WeaselError::UserEventUnpackingError(
            "empty UserEventPacker".into(),
        ))
    }

    fn flattened(event: Box<dyn Event<R>>) -> WeaselResult<Self, R> {
        Err(WeaselError::UserEventPackingError(
            event.clone(),
            "empty UserEventPacker".into(),
        ))
    }
}