//! Main entity in the game.

use crate::ability::{AbilitiesSeed, Ability, AbilityId};
use crate::actor::{Actor, ActorRules};
use crate::battle::{Battle, BattleRules, Checkpoint};
use crate::character::{Character, CharacterRules, Statistic, StatisticId, StatisticsSeed};
use crate::entity::{Entity, EntityId};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::system::*;
use crate::round::RoundState;
use crate::space::{Position, PositionClaim};
use crate::team::{EntityAddition, TeamId, TeamRules};
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result};

/// Type to represent the id of creatures.
pub type CreatureId<R> = <<R as BattleRules>::CR as CharacterRules<R>>::CreatureId;

type Statistics<R> = HashMap<
    <<<R as BattleRules>::CR as CharacterRules<R>>::Statistic as Id>::Id,
    <<R as BattleRules>::CR as CharacterRules<R>>::Statistic,
>;

type Abilities<R> = HashMap<
    <<<R as BattleRules>::AR as ActorRules<R>>::Ability as Id>::Id,
    <<R as BattleRules>::AR as ActorRules<R>>::Ability,
>;

/// A creature is the main acting entity of a battle.
///
/// Creatures can activate abilities during their round, occupy a spatial position and
/// are characterized by their statistics.
pub struct Creature<R: BattleRules> {
    id: EntityId<R>,
    team_id: TeamId<R>,
    position: Position<R>,
    statistics: Statistics<R>,
    abilities: Abilities<R>,
}

impl<R: BattleRules> Creature<R> {
    /// Returns an iterator over abilities.
    pub fn abilities(&self) -> Values<AbilityId<R>, Ability<R>> {
        self.abilities.values()
    }

    /// Returns the ability with the given id.
    pub fn ability(&self, id: &AbilityId<R>) -> Option<&Ability<R>> {
        self.abilities.get(id)
    }

    pub(crate) fn set_team_id(&mut self, id: TeamId<R>) {
        self.team_id = id;
    }
}

impl<R: BattleRules> Id for Creature<R> {
    type Id = CreatureId<R>;

    fn id(&self) -> &CreatureId<R> {
        let EntityId::Creature(id) = &self.id;
        id
    }
}

impl<R: BattleRules> Entity<R> for Creature<R> {
    fn entity_id(&self) -> &EntityId<R> {
        &self.id
    }

    fn position(&self) -> &Position<R> {
        &self.position
    }

    fn set_position(&mut self, position: Position<R>) {
        self.position = position;
    }
}

impl<R: BattleRules> Character<R> for Creature<R> {
    fn statistics<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Statistic<R>> + 'a> {
        Box::new(self.statistics.values())
    }

    fn statistic(&self, id: &StatisticId<R>) -> Option<&Statistic<R>> {
        self.statistics.get(id)
    }

    fn statistic_mut(&mut self, id: &StatisticId<R>) -> Option<&mut Statistic<R>> {
        self.statistics.get_mut(id)
    }

    fn add_statistic(&mut self, statistic: Statistic<R>) -> Option<Statistic<R>> {
        self.statistics.insert(statistic.id().clone(), statistic)
    }

    fn remove_statistic(&mut self, id: &StatisticId<R>) -> Option<Statistic<R>> {
        self.statistics.remove(id)
    }
}

impl<R: BattleRules> Actor<R> for Creature<R> {
    fn abilities<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Ability<R>> + 'a> {
        Box::new(self.abilities.values())
    }

    fn ability(&self, id: &AbilityId<R>) -> Option<&Ability<R>> {
        self.abilities.get(id)
    }

    fn ability_mut(&mut self, id: &AbilityId<R>) -> Option<&mut Ability<R>> {
        self.abilities.get_mut(id)
    }

    fn add_ability(&mut self, ability: Ability<R>) -> Option<Ability<R>> {
        self.abilities.insert(ability.id().clone(), ability)
    }

    fn remove_ability(&mut self, id: &AbilityId<R>) -> Option<Ability<R>> {
        self.abilities.remove(id)
    }

    fn team_id(&self) -> &TeamId<R> {
        &self.team_id
    }
}

/// Event to create a new creature.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct CreateCreature<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "CreatureId<R>: Serialize",
            deserialize = "CreatureId<R>: Deserialize<'de>"
        ))
    )]
    id: CreatureId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    team_id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Position<R>: Serialize",
            deserialize = "Position<R>: Deserialize<'de>"
        ))
    )]
    position: Position<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<StatisticsSeed<R>>: Serialize",
            deserialize = "Option<StatisticsSeed<R>>: Deserialize<'de>"
        ))
    )]
    statistics_seed: Option<StatisticsSeed<R>>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<AbilitiesSeed<R>>: Serialize",
            deserialize = "Option<AbilitiesSeed<R>>: Deserialize<'de>"
        ))
    )]
    abilities_seed: Option<AbilitiesSeed<R>>,
}

impl<R: BattleRules> Debug for CreateCreature<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "CreateCreature {{ id: {:?}, team_id: {:?}, position: {:?}, \
             statistics_seed: {:?}, abilities_seed: {:?} }}",
            self.id, self.team_id, self.position, self.statistics_seed, self.abilities_seed
        )
    }
}

impl<R: BattleRules> Clone for CreateCreature<R> {
    fn clone(&self) -> Self {
        CreateCreature {
            id: self.id.clone(),
            team_id: self.team_id.clone(),
            position: self.position.clone(),
            statistics_seed: self.statistics_seed.clone(),
            abilities_seed: self.abilities_seed.clone(),
        }
    }
}

impl<R: BattleRules> CreateCreature<R> {
    /// Collects an iterator into an hashmap.
    /// Subsequent values with same key are ignored.
    fn collect_from_iter<I>(
        it: I,
    ) -> HashMap<<<I as Iterator>::Item as Id>::Id, <I as Iterator>::Item>
    where
        I: Iterator,
        <I as Iterator>::Item: Id,
    {
        let mut map = HashMap::new();
        for e in it {
            if !map.contains_key(e.id()) {
                map.insert(e.id().clone(), e);
            }
        }
        map
    }

    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: CreatureId<R>,
        team_id: TeamId<R>,
        position: Position<R>,
    ) -> CreateCreatureTrigger<'a, R, P> {
        CreateCreatureTrigger {
            processor,
            id,
            team_id,
            position,
            statistics_seed: None,
            abilities_seed: None,
        }
    }

    /// Returns the id of the creature to be created.
    pub fn id(&self) -> &CreatureId<R> {
        &self.id
    }

    /// Returns the team id of the creature to be created.
    pub fn team_id(&self) -> &TeamId<R> {
        &self.team_id
    }

    /// Returns the position that the creature will take.
    pub fn position(&self) -> &Position<R> {
        &self.position
    }

    /// Returns the seed to generate the creature's statistics.
    pub fn statistics_seed(&self) -> &Option<StatisticsSeed<R>> {
        &self.statistics_seed
    }

    /// Returns the seed to generate the creature's abilities.
    pub fn abilities_seed(&self) -> &Option<AbilitiesSeed<R>> {
        &self.abilities_seed
    }
}

impl<R: BattleRules + 'static> Event<R> for CreateCreature<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        let team = battle
            .entities()
            .team(&self.team_id)
            .ok_or_else(|| WeaselError::TeamNotFound(self.team_id.clone()))?;
        // Check if the team accepts a new creature.
        if !battle.rules().team_rules().allow_new_entity(
            &battle.state,
            &team,
            EntityAddition::CreatureSpawn,
        ) {
            return Err(WeaselError::NewCreatureUnaccepted(self.team_id.clone()));
        }
        // Check id duplication.
        if battle.entities().creature(&self.id).is_some() {
            return Err(WeaselError::DuplicatedCreature(self.id.clone()));
        }
        // Check position.
        if !battle.space().check_move(
            PositionClaim::Spawn(&EntityId::Creature(self.id.clone())),
            &self.position,
        ) {
            return Err(WeaselError::PositionError(None, self.position.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Statistics' generation is influenced by the given statistics_seed, if present.
        let it = battle.rules.character_rules().generate_statistics(
            &self.statistics_seed,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        let statistics = CreateCreature::<R>::collect_from_iter(it);
        // Abilities' generation is influenced by the given abilities_seed, if present.
        let it = battle.rules.actor_rules().generate_abilities(
            &self.abilities_seed,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        let abilities = CreateCreature::<R>::collect_from_iter(it);
        // Create the creature.
        let creature = Creature {
            id: EntityId::Creature(self.id.clone()),
            team_id: self.team_id.clone(),
            position: self.position.clone(),
            statistics,
            abilities,
        };
        // Take the position.
        battle.state.space.move_entity(
            PositionClaim::Spawn(&EntityId::Creature(self.id.clone())),
            &self.position,
            &mut battle.metrics.write_handle(),
        );
        // Notify the rounds module.
        battle.state.rounds.on_actor_added(
            &creature,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        // Add the creature to the actors.
        battle
            .state
            .entities
            .add_creature(creature)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
        // Update metrics.
        battle
            .metrics
            .write_handle()
            .add_system_u64(CREATURES_CREATED, 1)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
    }

    fn kind(&self) -> EventKind {
        EventKind::CreateCreature
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `CreateCreature` event.
pub struct CreateCreatureTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: CreatureId<R>,
    team_id: TeamId<R>,
    position: Position<R>,
    statistics_seed: Option<StatisticsSeed<R>>,
    abilities_seed: Option<AbilitiesSeed<R>>,
}

impl<'a, R, P> CreateCreatureTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the generation of this creature's statistics.
    pub fn statistics_seed(
        &'a mut self,
        seed: StatisticsSeed<R>,
    ) -> &'a mut CreateCreatureTrigger<'a, R, P> {
        self.statistics_seed = Some(seed);
        self
    }

    /// Adds a seed to drive the generation of this creature's abilities.
    pub fn abilities_seed(
        &'a mut self,
        seed: AbilitiesSeed<R>,
    ) -> &'a mut CreateCreatureTrigger<'a, R, P> {
        self.abilities_seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for CreateCreatureTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `CreateCreature` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(CreateCreature {
            id: self.id.clone(),
            team_id: self.team_id.clone(),
            position: self.position.clone(),
            statistics_seed: self.statistics_seed.clone(),
            abilities_seed: self.abilities_seed.clone(),
        })
    }
}

/// Event to switch a creature from its current team to another.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ConvertCreature<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "CreatureId<R>: Serialize",
            deserialize = "CreatureId<R>: Deserialize<'de>"
        ))
    )]
    creature_id: CreatureId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    team_id: TeamId<R>,
}

impl<R: BattleRules> ConvertCreature<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        creature_id: CreatureId<R>,
        team_id: TeamId<R>,
    ) -> ConvertCreatureTrigger<R, P> {
        ConvertCreatureTrigger {
            processor,
            creature_id,
            team_id,
        }
    }

    /// Returns the id of the creature to be converted.
    pub fn creature_id(&self) -> &CreatureId<R> {
        &self.creature_id
    }

    /// Returns the id of the team that this creature should join.
    pub fn team_id(&self) -> &TeamId<R> {
        &self.team_id
    }
}

impl<R: BattleRules> Debug for ConvertCreature<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ConvertCreature {{ creature_id: {:?}, team_id: {:?} }}",
            self.creature_id, self.team_id
        )
    }
}

impl<R: BattleRules> Clone for ConvertCreature<R> {
    fn clone(&self) -> Self {
        ConvertCreature {
            creature_id: self.creature_id.clone(),
            team_id: self.team_id.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ConvertCreature<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Verify if the creature exists.
        let creature = battle
            .entities()
            .creature(&self.creature_id)
            .ok_or_else(|| WeaselError::CreatureNotFound(self.creature_id.clone()))?;
        // Verify if the team accept the new creature.
        let team = battle
            .entities()
            .team(&self.team_id)
            .ok_or_else(|| WeaselError::TeamNotFound(self.team_id.clone()))?;
        if team.id() == creature.team_id() {
            return Err(WeaselError::InvalidCreatureConversion(
                self.team_id.clone(),
                self.creature_id.clone(),
            ));
        }
        if !battle.rules().team_rules().allow_new_entity(
            &battle.state,
            &team,
            EntityAddition::CreatureConversion(&creature),
        ) {
            return Err(WeaselError::ConvertedCreatureUnaccepted(
                self.team_id.clone(),
                self.creature_id.clone(),
            ));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _event_queue: &mut Option<EventQueue<R>>) {
        battle
            .state
            .entities
            .convert_creature(&self.creature_id, &self.team_id)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
    }

    fn kind(&self) -> EventKind {
        EventKind::ConvertCreature
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ConvertCreature` event.
pub struct ConvertCreatureTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    creature_id: CreatureId<R>,
    team_id: TeamId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for ConvertCreatureTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ConvertCreature` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(ConvertCreature {
            creature_id: self.creature_id.clone(),
            team_id: self.team_id.clone(),
        })
    }
}

/// Event to remove a creature from the battle.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RemoveCreature<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "CreatureId<R>: Serialize",
            deserialize = "CreatureId<R>: Deserialize<'de>"
        ))
    )]
    id: CreatureId<R>,
}

impl<R: BattleRules> RemoveCreature<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: CreatureId<R>,
    ) -> RemoveCreatureTrigger<R, P> {
        RemoveCreatureTrigger { processor, id }
    }

    /// Returns the id of the creature to be removed.
    pub fn id(&self) -> &CreatureId<R> {
        &self.id
    }
}

impl<R: BattleRules> Debug for RemoveCreature<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "RemoveCreature {{ id: {:?} }}", self.id)
    }
}

impl<R: BattleRules> Clone for RemoveCreature<R> {
    fn clone(&self) -> Self {
        RemoveCreature {
            id: self.id.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RemoveCreature<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Verify if the creature exists.
        if battle.entities().creature(&self.id).is_none() {
            return Err(WeaselError::CreatureNotFound(self.id.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        let creature = battle
            .state
            .entities
            .creature(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: creature {:?} not found", self.id));
        // End the current round, if this creature was the actor.
        if let RoundState::Started(current_actor_id) = battle.state.rounds.state() {
            if current_actor_id == creature.entity_id() {
                // Invoke `RoundRules` callback.
                battle.state.rounds.on_end(
                    creature as &dyn Actor<_>,
                    &mut battle.entropy,
                    &mut battle.metrics.write_handle(),
                );
                // Check teams' objectives.
                Battle::check_objectives(
                    &battle.state,
                    &battle.rules.team_rules(),
                    &battle.metrics.read_handle(),
                    event_queue,
                    Checkpoint::RoundEnd,
                );
                // Set the round state.
                battle.state.rounds.set_state(RoundState::Ready);
            }
        }
        // Remove the creature.
        battle
            .state
            .entities
            .remove_creature(&self.id)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
    }

    fn kind(&self) -> EventKind {
        EventKind::RemoveCreature
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RemoveCreature` event.
pub struct RemoveCreatureTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: CreatureId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for RemoveCreatureTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RemoveCreature` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(RemoveCreature {
            id: self.id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleRules;
    use crate::rules::ability::SimpleAbility;
    use crate::rules::statistic::SimpleStatistic;
    use crate::util::tests::{creature, server, team};
    use crate::{battle_rules, rules::empty::*};
    use crate::{battle_rules_with_actor, battle_rules_with_character};

    #[derive(Default)]
    pub struct CustomCharacterRules {}

    impl<R: BattleRules> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type Statistic = SimpleStatistic<u32, u32>;
        type StatisticsSeed = ();
        type StatisticsAlteration = ();
    }

    #[test]
    fn mutable_statistics() {
        battle_rules_with_character! { CustomCharacterRules }
        // Create a battle.
        let mut server = server(CustomRules::new());
        team(&mut server, 1);
        creature(&mut server, 1, 1, ());
        let creature = server.battle.state.entities.creature_mut(&1).unwrap();
        assert!(creature.statistic(&1).is_none());
        creature.add_statistic(SimpleStatistic::new(1, 50));
        assert!(creature.statistic(&1).is_some());
        creature.statistic_mut(&1).unwrap().set_value(25);
        assert_eq!(creature.statistic(&1).unwrap().value(), 25);
        creature.remove_statistic(&1);
        assert!(creature.statistic(&1).is_none());
    }

    #[derive(Default)]
    pub struct CustomActorRules {}

    impl<R: BattleRules> ActorRules<R> for CustomActorRules {
        type Ability = SimpleAbility<u32, u32>;
        type AbilitiesSeed = ();
        type Activation = ();
        type AbilitiesAlteration = ();
    }

    #[test]
    fn mutable_abilities() {
        battle_rules_with_actor! { CustomActorRules }
        // Create a battle.
        let mut server = server(CustomRules::new());
        team(&mut server, 1);
        creature(&mut server, 1, 1, ());
        let creature = server.battle.state.entities.creature_mut(&1).unwrap();
        assert!(creature.ability(&1).is_none());
        creature.add_ability(SimpleAbility::new(1, 50));
        assert!(creature.ability(&1).is_some());
        creature.ability_mut(&1).unwrap().set_power(100);
        assert_eq!(creature.ability(&1).unwrap().power(), 100);
        creature.remove_ability(&1);
        assert!(creature.ability(&1).is_none());
    }
}
