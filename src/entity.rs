//! Module for entities and their storage.

use crate::actor::Actor;
use crate::battle::BattleRules;
use crate::character::Character;
use crate::creature::{Creature, CreatureId, RemoveCreature};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{EventProcessor, EventTrigger};
use crate::space::Position;
use crate::team::{Conclusion, Relation, RelationshipPair, Team, TeamId};
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result};

/// An entity represents any being existing in the game world.
pub trait Entity<R: BattleRules> {
    /// Returns the id of this entity.
    fn entity_id(&self) -> &EntityId<R>;

    /// Returns this entity position.
    fn position(&self) -> &Position<R>;

    /// Sets a new position for this entity.
    fn set_position(&mut self, position: Position<R>);
}

/// Id to uniquely identify an entity.
/// `EntityId` is used as global id to identify entities in the game world
/// regardless of their type.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum EntityId<R: BattleRules> {
    /// Standard creature.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "CreatureId<R>: Serialize",
            deserialize = "CreatureId<R>: Deserialize<'de>"
        ))
    )]
    Creature(CreatureId<R>),
}

impl<R: BattleRules> EntityId<R> {
    /// Returns this entity id refers to an object that satisfies the `Character` trait.
    pub(crate) fn is_character(&self) -> bool {
        match self {
            EntityId::Creature(_) => true,
        }
    }

    /// Returns this entity id refers to an object that satisfies the `Actor` trait.
    pub(crate) fn is_actor(&self) -> bool {
        match self {
            EntityId::Creature(_) => true,
        }
    }

    /// Extracts a creature id out of this entity id.
    ///
    /// Returns an error if the entity id's type is not creature.
    pub fn creature(&self) -> WeaselResult<CreatureId<R>, R> {
        match self {
            EntityId::Creature(id) => Ok(id.clone()),
        }
    }
}

impl<R: BattleRules> Debug for EntityId<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            EntityId::Creature(id) => write!(f, "EntityId::Creature {{ {:?} }}", id),
        }
    }
}

impl<R: BattleRules> Copy for EntityId<R> where CreatureId<R>: Copy {}

impl<R: BattleRules> Display for EntityId<R>
where
    CreatureId<R>: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            EntityId::Creature(id) => write!(f, "Creature ({})", id),
        }
    }
}

impl<R: BattleRules> Clone for EntityId<R> {
    fn clone(&self) -> Self {
        match self {
            EntityId::Creature(id) => EntityId::Creature(id.clone()),
        }
    }
}

impl<R: BattleRules> PartialEq<EntityId<R>> for EntityId<R> {
    fn eq(&self, other: &EntityId<R>) -> bool {
        match self {
            EntityId::Creature(id) => match other {
                EntityId::Creature(other_id) => id == other_id,
            },
        }
    }
}

/// Represents a change to an entity's existence.
pub enum Transmutation {
    /// Entity entirely removed from the battle.
    REMOVAL,
}

/// Triggers an event to transmute an entity.
pub(crate) fn transmute_entity<R, P>(
    id: &EntityId<R>,
    transmutation: Transmutation,
    processor: &mut P,
) where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    match transmutation {
        Transmutation::REMOVAL => match id {
            EntityId::Creature(id) => {
                RemoveCreature::trigger(processor, id.clone()).fire();
            }
        },
    }
}

/// Data structure to manage ownership of teams and entities.
pub struct Entities<R: BattleRules> {
    teams: HashMap<TeamId<R>, Team<R>>,
    creatures: HashMap<CreatureId<R>, Creature<R>>,
    relations: HashMap<RelationshipPair<R>, Relation>,
}

impl<R: BattleRules> Entities<R> {
    pub(crate) fn new() -> Entities<R> {
        Entities {
            teams: HashMap::new(),
            creatures: HashMap::new(),
            relations: HashMap::new(),
        }
    }

    /// Returns an iterator over creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &Creature<R>> {
        self.creatures.values()
    }

    /// Returns the creature with the given id.
    pub fn creature(&self, id: &CreatureId<R>) -> Option<&Creature<R>> {
        self.creatures.get(id)
    }

    /// Returns a mutable reference to the creature with the given id.
    pub(crate) fn creature_mut(&mut self, id: &CreatureId<R>) -> Option<&mut Creature<R>> {
        self.creatures.get_mut(id)
    }

    /// Returns the team with the given id.
    pub fn team(&self, id: &TeamId<R>) -> Option<&Team<R>> {
        self.teams.get(id)
    }

    /// Returns a mutable reference to the team with the given id.
    pub(crate) fn team_mut(&mut self, id: &TeamId<R>) -> Option<&mut Team<R>> {
        self.teams.get_mut(id)
    }

    /// Returns an iterator over teams.
    pub fn teams(&self) -> impl Iterator<Item = &Team<R>> {
        self.teams.values()
    }

    pub(crate) fn add_team(&mut self, team: Team<R>) {
        self.teams.insert(team.id().clone(), team);
    }

    pub(crate) fn add_creature(&mut self, creature: Creature<R>) -> WeaselResult<(), R> {
        // Update team's creature list.
        let team = self
            .teams
            .get_mut(&creature.team_id())
            .ok_or_else(|| WeaselError::TeamNotFound(creature.team_id().clone()))?;
        team.creatures_mut().push(creature.id().clone());
        // Insert the creature.
        self.creatures.insert(creature.id().clone(), creature);
        Ok(())
    }

    /// Returns an iterator over entities.
    pub fn entities(&self) -> impl Iterator<Item = &dyn Entity<R>> {
        self.creatures.values().map(|e| e as &dyn Entity<R>)
    }

    /// Returns a mutable iterator over entities.
    pub fn entities_mut(&mut self) -> impl Iterator<Item = &mut dyn Entity<R>> {
        self.creatures.values_mut().map(|e| e as &mut dyn Entity<R>)
    }

    /// Returns the entity with the given id.
    pub fn entity(&self, id: &EntityId<R>) -> Option<&dyn Entity<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Entity<R>),
        }
    }

    /// Returns a mutable reference to the entity with the given id.
    pub(crate) fn entity_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Entity<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Entity<R>),
        }
    }

    /// Returns an iterator over characters.
    pub fn characters(&self) -> impl Iterator<Item = &dyn Character<R>> {
        self.creatures
            .values()
            .filter(|e| e.entity_id().is_character())
            .map(|e| e as &dyn Character<R>)
    }

    /// Returns the character with the given id.
    pub fn character(&self, id: &EntityId<R>) -> Option<&dyn Character<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Character<R>),
        }
    }

    /// Returns a mutable reference to the character with the given id.
    pub(crate) fn character_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Character<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Character<R>),
        }
    }

    /// Returns an iterator over actors.
    pub fn actors(&self) -> impl Iterator<Item = &dyn Actor<R>> {
        self.creatures
            .values()
            .filter(|e| e.entity_id().is_actor())
            .map(|e| e as &dyn Actor<R>)
    }

    /// Returns the character with the given id.
    pub fn actor(&self, id: &EntityId<R>) -> Option<&dyn Actor<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Actor<R>),
        }
    }

    /// Returns a mutable reference to the actor with the given id.
    pub(crate) fn actor_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Actor<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Actor<R>),
        }
    }

    /// Updates current relations by merging them with `new_relations`.
    /// Existing relations are overridden.
    pub(crate) fn update_relations(&mut self, relations: Vec<(RelationshipPair<R>, Relation)>) {
        for (pair, relation) in relations {
            self.relations.insert(pair, relation);
        }
    }

    /// Returns the `Relation` between two teams. Relations are symmetric.
    ///
    /// The relation of a team towards itself is `Kin`.
    pub fn relation(&self, first: &TeamId<R>, second: &TeamId<R>) -> Option<Relation> {
        if first == second {
            Some(Relation::Kin)
        } else {
            self.relations
                .get(&RelationshipPair::new(first.clone(), second.clone()))
                .copied()
        }
    }

    /// Returns all allied teams' id of a team.
    pub fn allies_id<'a>(&'a self, id: &'a TeamId<R>) -> impl Iterator<Item = TeamId<R>> + 'a {
        self.relations
            .iter()
            .filter(move |&(k, _)| k.first == *id || k.second == *id)
            .filter(|&(_, v)| *v == Relation::Ally)
            .map(|(k, _)| k.values())
            .flatten()
            .filter(move |v| v != id)
    }

    /// Returns all allied teams of a team.
    pub fn allies<'a>(&'a self, id: &'a TeamId<R>) -> impl Iterator<Item = &Team<R>> + 'a {
        self.allies_id(id).map(move |id| self.team(&id).unwrap())
    }

    /// Returns all enemy teams' id of a team.
    pub fn enemies_id<'a>(&'a self, id: &'a TeamId<R>) -> impl Iterator<Item = TeamId<R>> + 'a {
        self.relations
            .iter()
            .filter(move |&(k, _)| k.first == *id || k.second == *id)
            .filter(|&(_, v)| *v == Relation::Enemy)
            .map(|(k, _)| k.values())
            .flatten()
            .filter(move |v| v != id)
    }

    /// Returns all enemy teams of a team.
    pub fn enemies<'a>(&'a self, id: &'a TeamId<R>) -> impl Iterator<Item = &Team<R>> + 'a {
        self.enemies_id(id).map(move |id| self.team(&id).unwrap())
    }

    /// Returns all victorious teams.
    pub fn victorious(&self) -> impl Iterator<Item = &Team<R>> {
        self.teams
            .values()
            .filter(|&team| team.conclusion() == Some(Conclusion::Victory))
    }

    /// Returns the id of all victorious teams.
    pub fn victorious_id(&self) -> impl Iterator<Item = TeamId<R>> + '_ {
        self.victorious().map(|team| team.id().clone())
    }

    /// Returns all defeated teams.
    pub fn defeated(&self) -> impl Iterator<Item = &Team<R>> {
        self.teams
            .values()
            .filter(|&team| team.conclusion() == Some(Conclusion::Defeat))
    }

    /// Returns the id of all defeated teams.
    pub fn defeated_id(&self) -> impl Iterator<Item = TeamId<R>> + '_ {
        self.defeated().map(|team| team.id().clone())
    }

    /// Removes a creature from the battle. The creature must exist.
    ///
    /// Returns the removed creature.
    pub(crate) fn remove_creature(&mut self, id: &CreatureId<R>) -> WeaselResult<Creature<R>, R> {
        // Extract the creature.
        let creature = self
            .creatures
            .remove(id)
            .ok_or_else(|| WeaselError::CreatureNotFound(id.clone()))?;
        // Remove the creature's id from the team list of creatures.
        let team = self
            .teams
            .get_mut(creature.team_id())
            .ok_or_else(|| WeaselError::TeamNotFound(creature.team_id().clone()))?;
        team.remove_creature(id);
        Ok(creature)
    }

    /// Changes a creature's team.
    pub(crate) fn convert_creature(
        &mut self,
        creature_id: &CreatureId<R>,
        team_id: &TeamId<R>,
    ) -> WeaselResult<(), R> {
        let creature = self
            .creatures
            .get_mut(&creature_id)
            .ok_or_else(|| WeaselError::CreatureNotFound(creature_id.clone()))?;
        let current_team_id = creature.team_id().clone();
        // Change the original team's creature lists.
        let current_team = self
            .teams
            .get_mut(&current_team_id)
            .ok_or_else(|| WeaselError::TeamNotFound(current_team_id))?;
        current_team.remove_creature(creature_id);
        // Change the new team's creature lists.
        let new_team = self
            .teams
            .get_mut(&team_id)
            .ok_or_else(|| WeaselError::TeamNotFound(team_id.clone()))?;
        new_team.creatures_mut().push(creature_id.clone());
        // Change the creature's team.
        creature.set_team_id(team_id.clone());
        Ok(())
    }

    /// Removes a team from the battle. The team must exist and be empty.
    ///
    /// Returns the removed team.
    pub(crate) fn remove_team(&mut self, id: &TeamId<R>) -> WeaselResult<Team<R>, R> {
        // Check preconditions.
        let team = self
            .teams
            .get(id)
            .ok_or_else(|| WeaselError::TeamNotFound(id.clone()))?;
        if team.creatures().peekable().peek().is_some() {
            return Err(WeaselError::TeamNotEmpty(id.clone()));
        }
        // Extract the team.
        let team = self
            .teams
            .remove(id)
            .ok_or_else(|| WeaselError::TeamNotFound(id.clone()))?;
        Ok(team)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{battle_rules, rules::empty::*};

    battle_rules! {}

    #[test]
    fn entity_id_equality() {
        assert_eq!(
            EntityId::<CustomRules>::Creature(5),
            EntityId::<CustomRules>::Creature(5)
        );
        assert_ne!(
            EntityId::<CustomRules>::Creature(5),
            EntityId::<CustomRules>::Creature(6)
        );
    }
}
