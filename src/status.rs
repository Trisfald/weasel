//! Module for long lasting status effects.

use crate::battle::{Battle, BattleRules};
use crate::character::verify_is_character;
use crate::entity::EntityId;
use crate::error::WeaselResult;
use crate::event::{Event, EventId, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::fight::FightRules;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{Debug, Formatter, Result};

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
            origin: None,
        }
    }

    /// Creates a new `LinkedStatus` with an origin.
    pub fn with_origin(status: Status<R>, origin: EventId) -> LinkedStatus<R> {
        LinkedStatus {
            status,
            origin: Some(origin),
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

/// An event to inflict a status effect on a character.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleRules};
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::status::InflictStatus;
/// use weasel::{Server, battle_rules, rules::empty::*};
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// todo
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct InflictStatus<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "EntityId<R>: Serialize",
            deserialize = "EntityId<R>: Deserialize<'de>"
        ))
    )]
    entity_id: EntityId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "StatusId<R>: Serialize",
            deserialize = "StatusId<R>: Deserialize<'de>"
        ))
    )]
    status_id: StatusId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<Potency<R>>: Serialize",
            deserialize = "Option<Potency<R>>: Deserialize<'de>"
        ))
    )]
    potency: Option<Potency<R>>,
}

impl<R: BattleRules> InflictStatus<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        entity_id: EntityId<R>,
        status_id: StatusId<R>,
    ) -> InflictStatusTrigger<'a, R, P> {
        InflictStatusTrigger {
            processor,
            entity_id,
            status_id,
            potency: None,
        }
    }

    /// Returns the id of the entity target of this status.
    pub fn entity_id(&self) -> &EntityId<R> {
        &self.entity_id
    }

    /// Returns the id of the status to be inflicted.
    pub fn status_id(&self) -> &StatusId<R> {
        &self.status_id
    }

    /// Returns the status' potency.
    pub fn potency(&self) -> &Option<Potency<R>> {
        &self.potency
    }
}

impl<R: BattleRules> Debug for InflictStatus<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "InflictStatus {{ entity_id: {:?}, status_id: {:?}, potency: {:?} }}",
            self.entity_id, self.status_id, self.potency
        )
    }
}

impl<R: BattleRules> Clone for InflictStatus<R> {
    fn clone(&self) -> Self {
        InflictStatus {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
            potency: self.potency.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for InflictStatus<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_is_character(battle.entities(), &self.entity_id)
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        // TODO
    }

    fn kind(&self) -> EventKind {
        EventKind::InflictStatus
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `InflictStatus` event.
pub struct InflictStatusTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    entity_id: EntityId<R>,
    status_id: StatusId<R>,
    potency: Option<Potency<R>>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for InflictStatusTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `InflictStatus` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(InflictStatus {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
            potency: self.potency.clone(),
        })
    }
}
