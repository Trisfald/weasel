//! Module for long lasting status effects.

use crate::battle::BattleRules;
use crate::event::EventId;
use crate::fight::FightRules;
use crate::util::Id;

/// A long lasting effect altering an entity's condition.
///
/// Statuses are used to represent anything that changes at least one property of an entity,
/// for a given amont of time. DoT (damage over time) are one example.\
/// A status can alter an entity just once or at every round.
pub type Status<R> = <<R as BattleRules>::FR as FightRules<R>>::Status;

/// Alias for `Status<R>::Id`.
pub type StatusId<R> = <Status<R> as Id>::Id;

/// Represents the intensity of a status.
pub type Potency<R> = <<R as BattleRules>::FR as FightRules<R>>::Potency;

/// Stores a `Status` and an optional link to its origin.
pub struct LinkedStatus<R: BattleRules> {
    status: Status<R>,
    origin: Option<EventId>,
}

impl<R: BattleRules> LinkedStatus<R> {
    /// Creates a new `LinkedStatus` without any origin.
    pub fn new(status: Status<R>) -> LinkedStatus<R> {
        LinkedStatus {
            status,
            origin: None
        }
    }

    /// Creates a new `LinkedStatus` with an origin.
    pub fn with_origin(status: Status<R>, origin: EventId) -> LinkedStatus<R> {
        LinkedStatus {
            status,
            origin: Some(origin)
        }
    }

    /// Returns a reference to the status.
    pub fn status(&self) -> &Status<R> {
        &self.status
    }

    /// Returns a mutable reference to the status.
    pub fn status_mut(&mut self) -> &mut Status<R> {
        &mut self.status
    }

    /// Returns the origin event's id of this status.
    pub fn origin(&self) -> Option<EventId> {
        self.origin
    }
}

impl<R: BattleRules> std::ops::Deref for LinkedStatus<R> {
    type Target = Status<R>;

    fn deref(&self) -> &Self::Target {
        &self.status
    }
}

impl<R: BattleRules> std::ops::DerefMut for LinkedStatus<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.status
    }
}
