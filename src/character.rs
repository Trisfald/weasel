//! Character rules.

use crate::battle::{Battle, BattleRules};
use crate::entity::{transmute_entity, Entities, Entity, EntityId, Transmutation};
use crate::entropy::Entropy;
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger, Prioritized};
use crate::metric::WriteMetrics;
use crate::status::{AppliedStatus, Potency, Status, StatusId};
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{Debug, Formatter, Result};
use std::hash::Hash;

/// Rules to define the structure and the behavior of characters.
pub trait CharacterRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [CreatureId](../creature/type.CreatureId.html).
    type CreatureId: Hash + Eq + Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [CreatureId](../creature/type.CreatureId.html).
    type CreatureId: Hash + Eq + Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [ObjectId](../object/type.ObjectId.html).
    type ObjectId: Hash + Eq + Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [ObjectId](../object/type.ObjectId.html).
    type ObjectId: Hash + Eq + Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// See [Statistic](type.Statistic.html).
    type Statistic: Id + 'static;

    #[cfg(not(feature = "serialization"))]
    /// See [StatisticsSeed](type.StatisticsSeed.html).
    type StatisticsSeed: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [StatisticsSeed](type.StatisticsSeed.html).
    type StatisticsSeed: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [StatisticsAlteration](type.StatisticsAlteration.html).
    type StatisticsAlteration: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [StatisticsAlteration](type.StatisticsAlteration.html).
    type StatisticsAlteration: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// Generates all statistics of a character.
    /// Statistics should have unique ids, otherwise only the last entry will be persisted.
    ///
    /// The provided implementation generates an empty set of statistics.
    fn generate_statistics(
        &self,
        _seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        Box::new(std::iter::empty())
    }

    /// Alters one or more statistics starting from the given alteration object.\
    /// Returns an optional `Transmutation` to be applied to the character as result of
    /// this alteration.
    ///
    /// The provided implementation does nothing.
    fn alter_statistics(
        &self,
        _character: &mut dyn Character<R>,
        _alteration: &Self::StatisticsAlteration,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Option<Transmutation> {
        None
    }

    /// Generates a status to be applied to the given character.\
    /// Returns the new status or nothing if no status should be added. Existing status with
    /// the same id will be replaced.
    ///
    /// The provided implementation returns `None`.
    fn generate_status(
        &self,
        _character: &dyn Character<R>,
        _status_id: &StatusId<R>,
        _potency: &Option<Potency<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Option<Status<R>> {
        None
    }
}

/// Type to represent an individual statistic.
///
/// Statistics are the primary way to describe attributes of characters. For instance,
/// the *health points* of a creature are a prime example of statistic.
pub type Statistic<R> = <<R as BattleRules>::CR as CharacterRules<R>>::Statistic;

/// Alias for `Statistic<R>::Id`.
pub type StatisticId<R> = <Statistic<R> as Id>::Id;

/// Type to drive the generation of all statistics of a character.
pub type StatisticsSeed<R> = <<R as BattleRules>::CR as CharacterRules<R>>::StatisticsSeed;

/// Encapsulates the data used to describe an alteration of one or more statistics.
pub type StatisticsAlteration<R> =
    <<R as BattleRules>::CR as CharacterRules<R>>::StatisticsAlteration;

/// A trait for objects which possess statistics.
pub trait Character<R: BattleRules>: Entity<R> {
    /// Returns an iterator over statistics.
    fn statistics<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Statistic<R>> + 'a>;

    /// Returns a mutable iterator over statistics.
    fn statistics_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Statistic<R>> + 'a>;

    /// Returns the statistic with the given id.
    fn statistic(&self, id: &StatisticId<R>) -> Option<&Statistic<R>>;

    /// Returns a mutable reference to the statistic with the given id.
    fn statistic_mut(&mut self, id: &StatisticId<R>) -> Option<&mut Statistic<R>>;

    /// Adds a new statistic. Replaces an existing statistic with the same id.
    /// Returns the replaced statistic, if present.
    fn add_statistic(&mut self, statistic: Statistic<R>) -> Option<Statistic<R>>;

    /// Removes a statistic.
    /// Returns the removed statistic, if present.
    fn remove_statistic(&mut self, id: &StatisticId<R>) -> Option<Statistic<R>>;

    /// Returns an iterator over statuses.
    fn statuses<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AppliedStatus<R>> + 'a>;

    /// Returns a mutable iterator over statuses.
    fn statuses_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut AppliedStatus<R>> + 'a>;

    /// Returns the status with the given id.
    fn status(&self, id: &StatusId<R>) -> Option<&AppliedStatus<R>>;

    /// Returns a mutable reference to the status with the given id.
    fn status_mut(&mut self, id: &StatusId<R>) -> Option<&mut AppliedStatus<R>>;

    /// Adds a new status. Replaces an existing status with the same id.
    /// Returns the replaced status, if present.
    fn add_status(&mut self, status: AppliedStatus<R>) -> Option<AppliedStatus<R>>;

    /// Removes a status.
    /// Returns the removed status, if present.
    fn remove_status(&mut self, id: &StatusId<R>) -> Option<AppliedStatus<R>>;
}

/// An event to alter the statistics of a character.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleRules};
/// use weasel::character::AlterStatistics;
/// use weasel::creature::CreateCreature;
/// use weasel::entity::EntityId;
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::team::CreateTeam;
/// use weasel::{Server, battle_rules, rules::empty::*};
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
/// AlterStatistics::trigger(&mut server, EntityId::Creature(creature_id), alteration)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::AlterStatistics
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct AlterStatistics<R: BattleRules> {
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
            serialize = "StatisticsAlteration<R>: Serialize",
            deserialize = "StatisticsAlteration<R>: Deserialize<'de>"
        ))
    )]
    alteration: StatisticsAlteration<R>,
}

impl<R: BattleRules> AlterStatistics<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: EntityId<R>,
        alteration: StatisticsAlteration<R>,
    ) -> AlterStatisticsTrigger<'a, R, P> {
        AlterStatisticsTrigger {
            processor,
            id,
            alteration,
        }
    }

    /// Returns the character's entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the definition of the changes to the character's statistics.
    pub fn alteration(&self) -> &StatisticsAlteration<R> {
        &self.alteration
    }
}

impl<R: BattleRules> Debug for AlterStatistics<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "AlterStatistics {{ id: {:?}, alteration: {:?} }}",
            self.id, self.alteration
        )
    }
}

impl<R: BattleRules> Clone for AlterStatistics<R> {
    fn clone(&self) -> Self {
        AlterStatistics {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for AlterStatistics<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_get_character(battle.entities(), &self.id).map(|_| ())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        // Retrieve the character.
        let character = battle
            .state
            .entities
            .character_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: character {:?} not found", self.id));
        // Alter the character.
        let transmutation = battle.rules.character_rules().alter_statistics(
            character,
            &self.alteration,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        // Change the character's existence if needed.
        if let Some(transmutation) = transmutation {
            transmute_entity(
                &self.id,
                transmutation,
                &mut event_queue.as_mut().map(|queue| Prioritized::new(queue)),
            );
        }
    }

    fn kind(&self) -> EventKind {
        EventKind::AlterStatistics
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `AlterStatistics` event.
pub struct AlterStatisticsTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    alteration: StatisticsAlteration<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for AlterStatisticsTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `AlterStatistics` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(AlterStatistics {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        })
    }
}

/// An event to regenerate the statistics of a character.
///
/// A new set of statistics is created from a seed.\
/// - Statistics already present in the character won't be modified.
/// - Statistics that the character didn't have before will be added.
/// - Current character's statistics that are not present in the new set will be removed
///   from the character.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleRules};
/// use weasel::character::RegenerateStatistics;
/// use weasel::creature::CreateCreature;
/// use weasel::entity::EntityId;
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::team::CreateTeam;
/// use weasel::{Server, battle_rules, rules::empty::*};
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
/// RegenerateStatistics::trigger(&mut server, EntityId::Creature(creature_id))
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::RegenerateStatistics
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RegenerateStatistics<R: BattleRules> {
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
            serialize = "Option<StatisticsSeed<R>>: Serialize",
            deserialize = "Option<StatisticsSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<StatisticsSeed<R>>,
}

impl<R: BattleRules> RegenerateStatistics<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &'_ mut P,
        id: EntityId<R>,
    ) -> RegenerateStatisticsTrigger<'_, R, P> {
        RegenerateStatisticsTrigger {
            processor,
            id,
            seed: None,
        }
    }

    /// Returns the character's entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the seed to regenerate the character's statistics.
    pub fn seed(&self) -> &Option<StatisticsSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for RegenerateStatistics<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "RegenerateStatistics {{ id: {:?}, seed: {:?} }}",
            self.id, self.seed
        )
    }
}

impl<R: BattleRules> Clone for RegenerateStatistics<R> {
    fn clone(&self) -> Self {
        RegenerateStatistics {
            id: self.id.clone(),
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RegenerateStatistics<R> {
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
        // Generate a new set of statistics.
        let statistics: Vec<_> = battle
            .rules
            .character_rules()
            .generate_statistics(
                &self.seed,
                &mut battle.entropy,
                &mut battle.metrics.write_handle(),
            )
            .collect();
        let mut to_remove = Vec::new();
        // Remove all character's statistics not present in the new set.
        for statistic in character.statistics() {
            if statistics
                .iter()
                .find(|e| e.id() == statistic.id())
                .is_none()
            {
                to_remove.push(statistic.id().clone());
            }
        }
        for statistic_id in to_remove {
            character.remove_statistic(&statistic_id);
        }
        // Add all statistics present in the new set but not in the character.
        for statistic in statistics {
            if character.statistic(statistic.id()).is_none() {
                character.add_statistic(statistic);
            }
        }
    }

    fn kind(&self) -> EventKind {
        EventKind::RegenerateStatistics
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RegenerateStatistics` event.
pub struct RegenerateStatisticsTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    seed: Option<StatisticsSeed<R>>,
}

impl<'a, R, P> RegenerateStatisticsTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the regeneration of this character's statistics.
    pub fn seed(
        &'a mut self,
        seed: StatisticsSeed<R>,
    ) -> &'a mut RegenerateStatisticsTrigger<'a, R, P> {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for RegenerateStatisticsTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RegenerateStatistics` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(RegenerateStatistics {
            id: self.id.clone(),
            seed: self.seed.clone(),
        })
    }
}

/// Checks if an entity exists and is a character.
/// Returns the character if successful;
pub(crate) fn verify_get_character<'a, R>(
    entities: &'a Entities<R>,
    id: &EntityId<R>,
) -> WeaselResult<&'a dyn Character<R>, R>
where
    R: BattleRules,
{
    // Check if this entity claims to be a character.
    if !id.is_character() {
        return Err(WeaselError::NotACharacter(id.clone()));
    }
    // Check if the character exists.
    let character = entities
        .character(id)
        .ok_or_else(|| WeaselError::EntityNotFound(id.clone()))?;
    Ok(character)
}
