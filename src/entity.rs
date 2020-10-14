//! Module for entities and their storage.

use crate::actor::Actor;
use crate::battle::BattleRules;
use crate::character::Character;
use crate::creature::{Creature, CreatureId, RemoveCreature};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventProcessor, EventTrigger};
use crate::object::{Object, ObjectId, RemoveObject};
use crate::space::Position;
use crate::team::{Conclusion, Relation, RelationshipPair, Team, TeamId};
use crate::util::Id;
use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

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
    /// Inanimate object.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "ObjectId<R>: Serialize",
            deserialize = "ObjectId<R>: Deserialize<'de>"
        ))
    )]
    Object(ObjectId<R>),
}

impl<R: BattleRules> EntityId<R> {
    /// Returns whether this entity id refers to an object that satisfies the `Character` trait.
    pub fn is_character(&self) -> bool {
        match self {
            Self::Creature(_) => true,
            Self::Object(_) => true,
        }
    }

    /// Returns whether this entity id refers to an object that satisfies the `Actor` trait.
    pub fn is_actor(&self) -> bool {
        match self {
            Self::Creature(_) => true,
            Self::Object(_) => false,
        }
    }

    /// Extracts a creature id out of this entity id.
    ///
    /// Returns an error if the entity id's type is not `Creature`.
    pub fn creature(&self) -> WeaselResult<CreatureId<R>, R> {
        match self {
            Self::Creature(id) => Ok(id.clone()),
            Self::Object(_) => Err(WeaselError::NotACreature(self.clone())),
        }
    }

    /// Extracts an object id out of this entity id.
    ///
    /// Returns an error if the entity id's type is not `Object`.
    pub fn object(&self) -> WeaselResult<ObjectId<R>, R> {
        match self {
            Self::Creature(_) => Err(WeaselError::NotAnObject(self.clone())),
            Self::Object(id) => Ok(id.clone()),
        }
    }
}

impl<R: BattleRules> Debug for EntityId<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Creature(id) => write!(f, "EntityId::Creature {{ {:?} }}", id),
            Self::Object(id) => write!(f, "EntityId::Object {{ {:?} }}", id),
        }
    }
}

impl<R: BattleRules> Copy for EntityId<R>
where
    CreatureId<R>: Copy,
    ObjectId<R>: Copy,
{
}

impl<R: BattleRules> Display for EntityId<R>
where
    CreatureId<R>: Display,
    ObjectId<R>: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Creature(id) => write!(f, "Creature ({})", id),
            Self::Object(id) => write!(f, "Object ({})", id),
        }
    }
}

impl<R: BattleRules> Clone for EntityId<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Creature(id) => Self::Creature(id.clone()),
            Self::Object(id) => Self::Object(id.clone()),
        }
    }
}

impl<R: BattleRules> PartialEq<Self> for EntityId<R> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Creature(id) => match other {
                Self::Creature(other_id) => id == other_id,
                _ => false,
            },
            Self::Object(id) => match other {
                Self::Object(other_id) => id == other_id,
                _ => false,
            },
        }
    }
}

impl<R: BattleRules> Eq for EntityId<R> {}

impl<R: BattleRules> Hash for EntityId<R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Creature(id) => id.hash(state),
            Self::Object(id) => id.hash(state),
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
            EntityId::Object(id) => {
                RemoveObject::trigger(processor, id.clone()).fire();
            }
        },
    }
}

/// Data structure to manage ownership of teams and entities.
pub struct Entities<R: BattleRules> {
    teams: IndexMap<TeamId<R>, Team<R>>,
    creatures: IndexMap<CreatureId<R>, Creature<R>>,
    objects: IndexMap<ObjectId<R>, Object<R>>,
    relations: IndexMap<RelationshipPair<R>, Relation>,
}

impl<R: BattleRules> Entities<R> {
    pub(crate) fn new() -> Self {
        Self {
            teams: IndexMap::new(),
            creatures: IndexMap::new(),
            objects: IndexMap::new(),
            relations: IndexMap::new(),
        }
    }

    /// Returns an iterator over creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &Creature<R>> {
        self.creatures.values()
    }

    /// Returns a mutable iterator over creatures.
    pub fn creatures_mut(&mut self) -> impl Iterator<Item = &mut Creature<R>> {
        self.creatures.values_mut()
    }

    /// Returns the creature with the given id.
    pub fn creature(&self, id: &CreatureId<R>) -> Option<&Creature<R>> {
        self.creatures.get(id)
    }

    /// Returns a mutable reference to the creature with the given id.
    pub fn creature_mut(&mut self, id: &CreatureId<R>) -> Option<&mut Creature<R>> {
        self.creatures.get_mut(id)
    }

    /// Returns an iterator over objects.
    pub fn objects(&self) -> impl Iterator<Item = &Object<R>> {
        self.objects.values()
    }

    /// Returns a mutable iterator over objects.
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut Object<R>> {
        self.objects.values_mut()
    }

    /// Returns the object with the given id.
    pub fn object(&self, id: &ObjectId<R>) -> Option<&Object<R>> {
        self.objects.get(id)
    }

    /// Returns a mutable reference to the object with the given id.
    pub fn object_mut(&mut self, id: &ObjectId<R>) -> Option<&mut Object<R>> {
        self.objects.get_mut(id)
    }

    /// Returns an iterator over teams.
    pub fn teams(&self) -> impl Iterator<Item = &Team<R>> {
        self.teams.values()
    }

    /// Returns a mutable iterator over teams.
    pub fn teams_mut(&mut self) -> impl Iterator<Item = &mut Team<R>> {
        self.teams.values_mut()
    }

    /// Returns the team with the given id.
    pub fn team(&self, id: &TeamId<R>) -> Option<&Team<R>> {
        self.teams.get(id)
    }

    /// Returns a mutable reference to the team with the given id.
    pub fn team_mut(&mut self, id: &TeamId<R>) -> Option<&mut Team<R>> {
        self.teams.get_mut(id)
    }

    pub(crate) fn add_team(&mut self, team: Team<R>) {
        self.teams.insert(team.id().clone(), team);
    }

    pub(crate) fn add_creature(&mut self, creature: Creature<R>) -> WeaselResult<(), R> {
        // Update team's creature list.
        let team = self
            .teams
            .get_mut(creature.team_id())
            .ok_or_else(|| WeaselError::TeamNotFound(creature.team_id().clone()))?;
        team.creatures_mut().push(creature.id().clone());
        // Insert the creature.
        self.creatures.insert(creature.id().clone(), creature);
        Ok(())
    }

    pub(crate) fn add_object(&mut self, object: Object<R>) {
        // Insert the object.
        self.objects.insert(object.id().clone(), object);
    }

    /// Returns an iterator over entities.
    pub fn entities(&self) -> impl Iterator<Item = &dyn Entity<R>> {
        self.creatures
            .values()
            .map(|e| e as &dyn Entity<R>)
            .chain(self.objects.values().map(|e| e as &dyn Entity<R>))
    }

    /// Returns a mutable iterator over entities.
    pub fn entities_mut(&mut self) -> impl Iterator<Item = &mut dyn Entity<R>> {
        self.creatures
            .values_mut()
            .map(|e| e as &mut dyn Entity<R>)
            .chain(self.objects.values_mut().map(|e| e as &mut dyn Entity<R>))
    }

    /// Returns the entity with the given id.
    pub fn entity(&self, id: &EntityId<R>) -> Option<&dyn Entity<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Entity<R>),
            EntityId::Object(id) => self.object(id).map(|e| e as &dyn Entity<R>),
        }
    }

    /// Returns a mutable reference to the entity with the given id.
    pub fn entity_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Entity<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Entity<R>),
            EntityId::Object(id) => self.object_mut(id).map(|e| e as &mut dyn Entity<R>),
        }
    }

    /// Returns an iterator over characters.
    pub fn characters(&self) -> impl Iterator<Item = &dyn Character<R>> {
        self.creatures()
            .map(|e| e as &dyn Character<R>)
            .chain(self.objects().map(|e| e as &dyn Character<R>))
    }

    /// Returns a mutable iterator over characters.
    pub fn characters_mut(&mut self) -> impl Iterator<Item = &mut dyn Character<R>> {
        self.creatures
            .values_mut()
            .map(|e| e as &mut dyn Character<R>)
            .chain(
                self.objects
                    .values_mut()
                    .map(|e| e as &mut dyn Character<R>),
            )
    }

    /// Returns the character with the given id.
    pub fn character(&self, id: &EntityId<R>) -> Option<&dyn Character<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Character<R>),
            EntityId::Object(id) => self.object(id).map(|e| e as &dyn Character<R>),
        }
    }

    /// Returns a mutable reference to the character with the given id.
    pub fn character_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Character<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Character<R>),
            EntityId::Object(id) => self.object_mut(id).map(|e| e as &mut dyn Character<R>),
        }
    }

    /// Returns an iterator over actors.
    pub fn actors(&self) -> impl Iterator<Item = &dyn Actor<R>> {
        self.creatures().map(|e| e as &dyn Actor<R>)
    }

    /// Returns a mutable iterator over actors.
    pub fn actors_mut(&mut self) -> impl Iterator<Item = &mut dyn Actor<R>> {
        self.creatures_mut().map(|e| e as &mut dyn Actor<R>)
    }

    /// Returns the character with the given id.
    pub fn actor(&self, id: &EntityId<R>) -> Option<&dyn Actor<R>> {
        match id {
            EntityId::Creature(id) => self.creature(id).map(|e| e as &dyn Actor<R>),
            EntityId::Object(_) => None,
        }
    }

    /// Returns a mutable reference to the actor with the given id.
    pub fn actor_mut(&mut self, id: &EntityId<R>) -> Option<&mut dyn Actor<R>> {
        match id {
            EntityId::Creature(id) => self.creature_mut(id).map(|e| e as &mut dyn Actor<R>),
            EntityId::Object(_) => None,
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
            .get_mut(creature_id)
            .ok_or_else(|| WeaselError::CreatureNotFound(creature_id.clone()))?;
        let current_team_id = creature.team_id().clone();
        // Change the original team's creature lists.
        let current_team = self
            .teams
            .get_mut(&current_team_id)
            .ok_or(WeaselError::TeamNotFound(current_team_id))?;
        current_team.remove_creature(creature_id);
        // Change the new team's creature lists.
        let new_team = self
            .teams
            .get_mut(team_id)
            .ok_or_else(|| WeaselError::TeamNotFound(team_id.clone()))?;
        new_team.creatures_mut().push(creature_id.clone());
        // Change the creature's team.
        creature.set_team_id(team_id.clone());
        Ok(())
    }

    /// Removes an object from the battle. The object must exist.
    ///
    /// Returns the removed object.
    pub(crate) fn remove_object(&mut self, id: &ObjectId<R>) -> WeaselResult<Object<R>, R> {
        // Extract the object.
        let object = self
            .objects
            .remove(id)
            .ok_or_else(|| WeaselError::ObjectNotFound(id.clone()))?;
        Ok(object)
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

/// Helper to get an event trigger capable of removing an entity from the battle.\
/// It delegates the actual work to a `RemoveCreature` or a `RemoveObject`.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateCreature,
///     CreateTeam, EntityId, EventTrigger, RemoveEntity, Server,
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
/// let entity_id = EntityId::Creature(creature_id);
/// let position = ();
/// CreateCreature::trigger(&mut server, creature_id, team_id, position)
///     .fire()
///     .unwrap();
///
/// RemoveEntity::trigger(&mut server, entity_id).fire().unwrap();
/// assert_eq!(server.battle().entities().creatures().count(), 0);
/// ```
pub struct RemoveEntity<R> {
    _phantom: PhantomData<R>,
}

impl<R: BattleRules> RemoveEntity<R> {
    /// Returns a trigger for this event helper.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: EntityId<R>,
    ) -> RemoveEntityTrigger<R, P> {
        RemoveEntityTrigger { processor, id }
    }
}

/// Trigger to build and a fire the correct event to remove an entity.
pub struct RemoveEntityTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for RemoveEntityTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns the event able to remove the entity.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        match &self.id {
            EntityId::Creature(id) => RemoveCreature::trigger(&mut (), id.clone()).event(),
            EntityId::Object(id) => RemoveObject::trigger(&mut (), id.clone()).event(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::battle::BattleRules;
    use crate::entity::EntityId;
    use crate::util::tests::{creature, object, server, team};
    use crate::{battle_rules, rules::empty::*};

    const TEAM_1_ID: u32 = 1;
    const TEAM_ERR_ID: u32 = 99;
    const CREATURE_1_ID: u32 = 1;
    const CREATURE_2_ID: u32 = 2;
    const CREATURE_ERR_ID: u32 = 99;
    const OBJECT_1_ID: u32 = 1;
    const OBJECT_2_ID: u32 = 2;
    const OBJECT_ERR_ID: u32 = 99;
    const ENTITY_C1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    const ENTITY_O1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
    const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);

    battle_rules! {}

    /// Creates a scenario with two creatures and two objects.
    macro_rules! scenario {
        () => {{
            // Create the battle.
            let mut server = server(CustomRules::new());
            // Create a team.
            team(&mut server, TEAM_1_ID);
            // Create two creatures.
            creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
            creature(&mut server, CREATURE_2_ID, TEAM_1_ID, ());
            // Create two objects.
            object(&mut server, OBJECT_1_ID, ());
            object(&mut server, OBJECT_2_ID, ());
            server
        }};
    }

    #[test]
    fn retrieval_concrete() {
        let mut server = scenario!();
        // Get a mutable reference to entities.
        let entities = server.battle.entities_mut();
        // Creatures.
        assert_eq!(entities.creatures().count(), 2);
        assert_eq!(entities.creatures_mut().count(), 2);
        assert!(entities.creature(&CREATURE_1_ID).is_some());
        assert!(entities.creature_mut(&CREATURE_1_ID).is_some());
        assert!(entities.creature(&CREATURE_ERR_ID).is_none());
        assert!(entities.creature_mut(&CREATURE_ERR_ID).is_none());
        // Objects.
        assert_eq!(entities.objects().count(), 2);
        assert_eq!(entities.objects_mut().count(), 2);
        assert!(entities.object(&OBJECT_1_ID).is_some());
        assert!(entities.object_mut(&OBJECT_2_ID).is_some());
        assert!(entities.object(&OBJECT_ERR_ID).is_none());
        assert!(entities.object_mut(&OBJECT_ERR_ID).is_none());
        // Teams.
        assert_eq!(entities.teams().count(), 1);
        assert_eq!(entities.teams_mut().count(), 1);
        assert!(entities.team(&TEAM_1_ID).is_some());
        assert!(entities.team_mut(&TEAM_1_ID).is_some());
        assert!(entities.team(&TEAM_ERR_ID).is_none());
        assert!(entities.team_mut(&TEAM_ERR_ID).is_none());
    }

    #[test]
    fn retrieval_trait() {
        let mut server = scenario!();
        // Get a mutable reference to entities.
        let entities = server.battle.entities_mut();
        // Entities.
        assert_eq!(entities.entities().count(), 4);
        assert_eq!(entities.entities_mut().count(), 4);
        assert!(entities.entity(&ENTITY_C1_ID).is_some());
        assert!(entities.entity_mut(&ENTITY_C1_ID).is_some());
        assert!(entities.entity(&ENTITY_O1_ID).is_some());
        assert!(entities.entity_mut(&ENTITY_O1_ID).is_some());
        assert!(entities.entity(&ENTITY_ERR_ID).is_none());
        assert!(entities.entity_mut(&ENTITY_ERR_ID).is_none());
        // Characters.
        assert_eq!(entities.characters().count(), 4);
        assert_eq!(entities.characters_mut().count(), 4);
        assert!(entities.character(&ENTITY_C1_ID).is_some());
        assert!(entities.character_mut(&ENTITY_C1_ID).is_some());
        assert!(entities.character(&ENTITY_O1_ID).is_some());
        assert!(entities.character_mut(&ENTITY_O1_ID).is_some());
        assert!(entities.character(&ENTITY_ERR_ID).is_none());
        assert!(entities.character_mut(&ENTITY_ERR_ID).is_none());
        // Actors.
        assert_eq!(entities.actors().count(), 2);
        assert_eq!(entities.actors_mut().count(), 2);
        assert!(entities.actor(&ENTITY_C1_ID).is_some());
        assert!(entities.actor_mut(&ENTITY_C1_ID).is_some());
        assert!(entities.actor(&ENTITY_O1_ID).is_none());
        assert!(entities.actor_mut(&ENTITY_O1_ID).is_none());
        assert!(entities.actor(&ENTITY_ERR_ID).is_none());
        assert!(entities.actor_mut(&ENTITY_ERR_ID).is_none());
    }
}
