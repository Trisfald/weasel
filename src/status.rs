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
struct LinkedStatus<R: BattleRules> {
    status: Status<R>,
    origin: Option<EventId>,
}
