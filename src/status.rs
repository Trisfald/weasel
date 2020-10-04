//! Module for long lasting status effects.

use crate::battle::{Battle, BattleRules};
use crate::character::{verify_get_character, CharacterRules};
use crate::entity::EntityId;
use crate::error::{WeaselError, WeaselResult};
use crate::event::{
    Event, EventId, EventKind, EventProcessor, EventQueue, EventTrigger, LinkedQueue,
};
use crate::fight::FightRules;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{Debug, Formatter, Result};

/// A long lasting effect altering an entity's condition.
///
/// Statuses are used to represent anything that changes at least one property of an entity,
/// for a given amount of turns. DoTs (damage over time) are one example.\
/// A status can alter an entity just once or at every turn.
pub type Status<R> = <<R as BattleRules>::CR as CharacterRules<R>>::Status;

/// Alias for `Status<R>::Id`.
pub type StatusId<R> = <Status<R> as Id>::Id;

/// Represents the intensity of a status.
pub type Potency<R> = <<R as BattleRules>::FR as FightRules<R>>::Potency;

/// Contains the changes to apply an alteration to one or more statuses.
pub type StatusesAlteration<R> = <<R as BattleRules>::CR as CharacterRules<R>>::StatusesAlteration;

/// Type for duration of statuses (in number of turns).
pub type StatusDuration = EventId;

/// Stores a `Status` and additional information about it.
pub struct AppliedStatus<R: BattleRules> {
    /// The status.
    status: Status<R>,
    /// An optional link to the origin event.
    origin: Option<EventId>,
    /// How long this status have been running.
    duration: StatusDuration,
}

impl<R: BattleRules> AppliedStatus<R> {
    /// Creates a new `AppliedStatus` without any origin.
    pub fn new(status: Status<R>) -> Self {
        Self {
            status,
            origin: None,
            duration: 0,
        }
    }

    /// Creates a new `AppliedStatus` with an origin.
    pub fn with_origin(status: Status<R>, origin: EventId) -> Self {
        Self {
            status,
            origin: Some(origin),
            duration: 0,
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

    /// Returns for how many turns the status has been in place.\
    /// Duration is increased at every turn start.
    pub fn duration(&self) -> StatusDuration {
        self.duration
    }

    /// Increases the duration by one.
    pub(crate) fn update(&mut self) {
        self.duration += 1;
    }
}

impl<R: BattleRules> std::ops::Deref for AppliedStatus<R> {
    type Target = Status<R>;

    fn deref(&self) -> &Self::Target {
        &self.status
    }
}

impl<R: BattleRules> std::ops::DerefMut for AppliedStatus<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.status
    }
}

/// Alias of `Status<R>`, used for new incoming statuses.
pub type NewStatus<R> = AppliedStatus<R>;

/// Alias of `Status<R>`, used for replaced statuses.
pub type OldStatus<R> = AppliedStatus<R>;

/// Represents the application of a new status effect.
pub enum Application<'a, R: BattleRules> {
    /// A completely new status is applied.
    New(&'a NewStatus<R>),
    /// An existing status is replaced by a new one.
    Replacement(&'a OldStatus<R>, &'a NewStatus<R>),
}

/// Updates all statuses of a entity.
/// Returns an error if the entity doesn't exist or if it isn't a character.
pub(crate) fn update_statuses<R: BattleRules + 'static>(
    id: &EntityId<R>,
    battle: &mut Battle<R>,
    event_queue: &mut Option<EventQueue<R>>,
) -> WeaselResult<(), R> {
    // Update the duration of all statuses.
    let character = battle
        .state
        .entities
        .character_mut(id)
        .ok_or_else(|| WeaselError::EntityNotFound(id.clone()))?;
    for status in character.statuses_mut() {
        status.update();
    }
    // Apply the effects of all statuses.
    let character = battle
        .state
        .entities
        .character(id)
        .ok_or_else(|| WeaselError::EntityNotFound(id.clone()))?;
    for status in character.statuses() {
        let terminated = battle.rules.fight_rules().update_status(
            &battle.state,
            character,
            status,
            // Set the origin of all events caused by the status' update to the status own origin.
            &mut event_queue
                .as_mut()
                .map(|queue| LinkedQueue::new(queue, status.origin())),
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        if terminated {
            // Remove the status.
            ClearStatus::trigger(
                event_queue,
                character.entity_id().clone(),
                status.id().clone(),
            )
            .fire();
        }
    }
    Ok(())
}

/// Event to inflict a status effect on a character.
///
/// A status may apply side effects upon activation and each time the creature takes an action.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateCreature,
///     CreateTeam, EntityId, EventKind, EventTrigger, InflictStatus, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
/// let creature_id = 1;
/// let position = ();
/// CreateCreature::trigger(&mut server, creature_id, team_id, position)
///     .fire()
///     .unwrap();
///
/// let status_id = 1;
/// InflictStatus::trigger(&mut server, EntityId::Creature(creature_id), status_id)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::InflictStatus
/// );
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
        Self {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
            potency: self.potency.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for InflictStatus<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_get_character(battle.entities(), &self.entity_id).map(|_| ())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        // Retrieve the character.
        let character = battle
            .state
            .entities
            .character_mut(&self.entity_id)
            .unwrap_or_else(|| {
                panic!(
                    "constraint violated: character {:?} not found",
                    self.entity_id
                )
            });
        // Generate the status.
        let status = battle.rules.character_rules().generate_status(
            character,
            &self.status_id,
            &self.potency,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        if let Some(status) = status {
            // We can assume that the id of this event will be equal to history's next_id(),
            // because the id will be assigned just after this function returns.
            let origin = battle.history.next_id();
            // Add the status to the character.
            let old_status = character.add_status(AppliedStatus::with_origin(status, origin));
            // Retrieve the character again, but this time immutably borrowing battle.state.
            let character = battle
                .state
                .entities
                .character(&self.entity_id)
                .unwrap_or_else(|| {
                    panic!(
                        "constraint violated: character {:?} not found",
                        self.entity_id
                    )
                });
            // Apply the status' side effects.
            let status = character.status(&self.status_id).unwrap_or_else(|| {
                panic!(
                    "constraint violated: status {:?} not found in {:?}",
                    self.status_id, self.entity_id
                )
            });
            let application = if let Some(old) = old_status.as_ref() {
                Application::Replacement(old, status)
            } else {
                Application::New(status)
            };
            battle.rules.fight_rules().apply_status(
                &battle.state,
                character,
                application,
                event_queue,
                &mut battle.entropy,
                &mut battle.metrics.write_handle(),
            );
        }
    }

    fn kind(&self) -> EventKind {
        EventKind::InflictStatus
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
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

impl<'a, R, P> InflictStatusTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Specify the potency of the status.
    pub fn potency(&'a mut self, potency: Potency<R>) -> &'a mut Self {
        self.potency = Some(potency);
        self
    }
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
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(InflictStatus {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
            potency: self.potency.clone(),
        })
    }
}

/// Event to erase a status effect and its side effects from a character.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleRules, ClearStatus, CreateCreature,
///     CreateTeam, EntityId, EventKind, EventTrigger, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
/// let creature_id = 1;
/// let position = ();
/// CreateCreature::trigger(&mut server, creature_id, team_id, position)
///     .fire()
///     .unwrap();
///
/// let status_id = 1;
/// let result =
///     ClearStatus::trigger(&mut server, EntityId::Creature(creature_id), status_id).fire();
/// // In this case the event should return an error because the creature is not afflicted
/// // by the status.
/// assert!(result.is_err());
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ClearStatus<R: BattleRules> {
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
}

impl<R: BattleRules> ClearStatus<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        entity_id: EntityId<R>,
        status_id: StatusId<R>,
    ) -> ClearStatusTrigger<'a, R, P> {
        ClearStatusTrigger {
            processor,
            entity_id,
            status_id,
        }
    }

    /// Returns the id of the entity from whom the status will be cleared.
    pub fn entity_id(&self) -> &EntityId<R> {
        &self.entity_id
    }

    /// Returns the id of the status to be cleared.
    pub fn status_id(&self) -> &StatusId<R> {
        &self.status_id
    }
}

impl<R: BattleRules> Debug for ClearStatus<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ClearStatus {{ entity_id: {:?}, status_id: {:?} }}",
            self.entity_id, self.status_id
        )
    }
}

impl<R: BattleRules> Clone for ClearStatus<R> {
    fn clone(&self) -> Self {
        Self {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ClearStatus<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        let character = verify_get_character(battle.entities(), &self.entity_id)?;
        // The character must be afflicted by the status.
        if character.status(&self.status_id).is_none() {
            Err(WeaselError::StatusNotPresent(
                self.entity_id.clone(),
                self.status_id.clone(),
            ))
        } else {
            Ok(())
        }
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        // Retrieve the character.
        let character = battle
            .state
            .entities
            .character(&self.entity_id)
            .unwrap_or_else(|| {
                panic!(
                    "constraint violated: character {:?} not found",
                    self.entity_id
                )
            });
        // Delete the status' side effects.
        let status = character.status(&self.status_id).unwrap_or_else(|| {
            panic!(
                "constraint violated: status {:?} not found in {:?}",
                self.status_id, self.entity_id
            )
        });
        battle.rules.fight_rules().delete_status(
            &battle.state,
            character,
            status,
            event_queue,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        // Retrieve the character, this time through a mutable reference.
        let character = battle
            .state
            .entities
            .character_mut(&self.entity_id)
            .unwrap_or_else(|| {
                panic!(
                    "constraint violated: character {:?} not found",
                    self.entity_id
                )
            });
        // Remove the status from the character.
        character.remove_status(&self.status_id);
    }

    fn kind(&self) -> EventKind {
        EventKind::ClearStatus
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ClearStatus` event.
pub struct ClearStatusTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    entity_id: EntityId<R>,
    status_id: StatusId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for ClearStatusTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ClearStatus` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(ClearStatus {
            entity_id: self.entity_id.clone(),
            status_id: self.status_id.clone(),
        })
    }
}

/// An event to alter the statuses of a character.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, AlterStatuses, Battle, BattleController, BattleRules,
///     CreateCreature, CreateTeam, EntityId, EventKind, EventTrigger, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
/// let creature_id = 1;
/// let position = ();
/// CreateCreature::trigger(&mut server, creature_id, team_id, position)
///     .fire()
///     .unwrap();
///
/// let alteration = ();
/// AlterStatuses::trigger(&mut server, EntityId::Creature(creature_id), alteration)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::AlterStatuses
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct AlterStatuses<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "EntityId<R>: Serialize",
            deserialize = "EntityId<R>: Deserialize<'de>"
        ))
    )]
    id: EntityId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "StatusesAlteration<R>: Serialize",
            deserialize = "StatusesAlteration<R>: Deserialize<'de>"
        ))
    )]
    alteration: StatusesAlteration<R>,
}

impl<R: BattleRules> AlterStatuses<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: EntityId<R>,
        alteration: StatusesAlteration<R>,
    ) -> AlterStatusesTrigger<'a, R, P> {
        AlterStatusesTrigger {
            processor,
            id,
            alteration,
        }
    }

    /// Returns the character's entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the definition of the changes to the character's statuses.
    pub fn alteration(&self) -> &StatusesAlteration<R> {
        &self.alteration
    }
}

impl<R: BattleRules> Debug for AlterStatuses<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "AlterStatuses {{ id: {:?}, alteration: {:?} }}",
            self.id, self.alteration
        )
    }
}

impl<R: BattleRules> Clone for AlterStatuses<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for AlterStatuses<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_get_character(battle.entities(), &self.id).map(|_| ())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Retrieve the character.
        let character = battle
            .state
            .entities
            .character_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: character {:?} not found", self.id));
        // Alter the character.
        battle.rules.character_rules().alter_statuses(
            character,
            &self.alteration,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::AlterStatuses
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `AlterStatuses` event.
pub struct AlterStatusesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    alteration: StatusesAlteration<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for AlterStatusesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `AlterStatuses` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(AlterStatuses {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        })
    }
}
