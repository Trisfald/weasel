//! Module for the spatial dimension.

use crate::battle::{Battle, BattleRules};
use crate::entity::{Entities, Entity, EntityId};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::WriteMetrics;
use crate::round::Rounds;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{Debug, Formatter, Result};

/// This object takes care of everything related to space and movement in the battle.\
/// It verifies the consistency of every entity's position.
pub struct Space<R: BattleRules> {
    model: SpaceModel<R>,
    rules: R::SR,
}

impl<R: BattleRules> Space<R> {
    /// Creates a new space object.
    pub(crate) fn new(seed: Option<SpaceSeed<R>>, rules: R::SR) -> Self {
        Self {
            model: rules.generate_model(&seed),
            rules,
        }
    }

    /// See [check_move](trait.SpaceRules.html#method.check_move).
    pub(crate) fn check_move<'a>(
        &self,
        claim: PositionClaim<'a, R>,
        position: &Position<R>,
    ) -> WeaselResult<(), R> {
        self.rules.check_move(&self.model, claim, position)
    }

    /// See [move_entity](trait.SpaceRules.html#method.move_entity).
    pub(crate) fn move_entity<'a>(
        &mut self,
        claim: PositionClaim<'a, R>,
        position: Option<&Position<R>>,
        metrics: &mut WriteMetrics<R>,
    ) {
        self.rules
            .move_entity(&mut self.model, claim, position, metrics);
    }

    /// Returns the space model.
    /// It stores all data needed to retrieve and compute the position of entities.
    pub fn model(&self) -> &SpaceModel<R> {
        &self.model
    }

    /// Returns a mutable reference to the space model.
    pub fn model_mut(&mut self) -> &mut SpaceModel<R> {
        &mut self.model
    }

    /// Returns this space's rules.
    pub fn rules(&self) -> &R::SR {
        &self.rules
    }

    /// Returns a mutable reference to this space's rules.
    pub fn rules_mut(&mut self) -> &mut R::SR {
        &mut self.rules
    }
}

/// Rules to govern the space dimension in a game.
///
/// This rules are used to determine if an entity can occupy a given position and to keep a model
/// containing all entities' positions.
pub trait SpaceRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [Position](type.Position.html).
    type Position: Eq + Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [Position](type.Position.html).
    type Position: Eq + Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [SpaceSeed](type.SpaceSeed.html).
    type SpaceSeed: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [SpaceSeed](type.SpaceSeed.html).
    type SpaceSeed: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [SpaceAlteration](type.SpaceAlteration.html).
    type SpaceAlteration: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [SpaceAlteration](type.SpaceAlteration.html).
    type SpaceAlteration: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    /// See [SpaceModel](type.SpaceModel.html).
    type SpaceModel;

    /// Generates a `SpaceModel` starting from a `SpaceSeed`.
    fn generate_model(&self, seed: &Option<Self::SpaceSeed>) -> Self::SpaceModel;

    /// Checks if the given entity can occupy a new position.
    ///
    /// The claim tells in which context the entity is trying to acquire the position.
    ///
    /// The provided implementation accepts every move.
    fn check_move<'a>(
        &self,
        _model: &Self::SpaceModel,
        _claim: PositionClaim<'a, R>,
        _position: &Self::Position,
    ) -> WeaselResult<(), R> {
        Ok(())
    }

    /// Moves an entity into a new position.
    ///
    /// Position's correctness will be validated beforehand with `check_move`,
    /// unless it is `None`.
    /// An empty position means that the entity is disappearing from the battle.\
    /// The claim tells in which context the entity is trying to acquire the position.\
    ///
    /// The provided implementation does nothing.
    fn move_entity<'a>(
        &self,
        _model: &mut Self::SpaceModel,
        _claim: PositionClaim<'a, R>,
        _position: Option<&Self::Position>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Translates an entity from one space model to another one.
    ///
    /// This method must apply the necessary changes to the entity's position and to the new model
    /// so that positions consistency is preserved.
    ///
    /// The provided implementation does nothing.
    fn translate_entity(
        &self,
        _model: &Self::SpaceModel,
        _new_model: &mut Self::SpaceModel,
        _entity: &mut dyn Entity<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Changes the current space model, starting from the information contained in `alteration`.
    ///
    /// Consequences of this change should be applied by registering events inside `event_queue`.
    ///
    /// The provided implementation does nothing.
    fn alter_space(
        &self,
        _entities: &Entities<R>,
        _rounds: &Rounds<R>,
        _model: &mut Self::SpaceModel,
        _alteration: &Self::SpaceAlteration,
        _event_queue: &mut Option<EventQueue<R>>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }
}

/// Type to represent an object's position.
///
/// Position's meaning changes depending on your definition of space.\
/// Remember that positions should contain all information to fully represent what does
/// it mean to occupy a *piece* of the battlefield. For instance, if your entities occupy
/// an area, both the area's location and dimension must be encapsulated in this type.
pub type Position<R> = <<R as BattleRules>::SR as SpaceRules<R>>::Position;

/// Type to represent a space seed.
/// It is used to bootstrap the spatial model of a game.
pub type SpaceSeed<R> = <<R as BattleRules>::SR as SpaceRules<R>>::SpaceSeed;

/// Type to store all information about the space dimension in the game.
///
/// The space model is used to store the entities' position as well as the free accessible
/// space and all environment hazards.\
/// An example of space model is a matrix containing the position of all pieces in a game of chess.
pub type SpaceModel<R> = <<R as BattleRules>::SR as SpaceRules<R>>::SpaceModel;

/// Represents an alteration to the space model.
///
/// It is used to evolve the battle's current space model. This alteration should only contain
/// the direct changes to the model. All side effects deriving from such change have to be
/// implemented in the space rules `alter_space` method.
pub type SpaceAlteration<R> = <<R as BattleRules>::SR as SpaceRules<R>>::SpaceAlteration;

/// Represents an entity's claim to a given position.
pub enum PositionClaim<'a, R: BattleRules> {
    /// The entity is spawning.
    Spawn(&'a EntityId<R>),
    /// The entity wants to change its position.
    Movement(&'a dyn Entity<R>),
}

impl<R: BattleRules> PositionClaim<'_, R> {
    /// Returns the id of the entity behind this claim.
    pub fn entity_id(&self) -> &EntityId<R> {
        match self {
            Self::Spawn(id) => id,
            Self::Movement(entity) => entity.entity_id(),
        }
    }
}

/// An event to move an entity from its position to a new one.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleController, BattleRules};
/// use weasel::creature::CreateCreature;
/// use weasel::entity::EntityId;
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::space::MoveEntity;
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
/// MoveEntity::trigger(&mut server, EntityId::Creature(creature_id), position)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::MoveEntity
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct MoveEntity<R: BattleRules> {
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
            serialize = "Position<R>: Serialize",
            deserialize = "Position<R>: Deserialize<'de>"
        ))
    )]
    position: Position<R>,
}

impl<R: BattleRules> MoveEntity<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: EntityId<R>,
        position: Position<R>,
    ) -> MoveEntityTrigger<'a, R, P> {
        MoveEntityTrigger {
            processor,
            id,
            position,
        }
    }

    /// Returns the entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the new position to be set for the entity.
    pub fn position(&self) -> &Position<R> {
        &self.position
    }
}

impl<R: BattleRules> Debug for MoveEntity<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "MoveEntity {{ creature_id: {:?}, position: {:?} }}",
            self.id, self.position
        )
    }
}

impl<R: BattleRules> Clone for MoveEntity<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            position: self.position.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for MoveEntity<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Find the entity.
        let entity = battle
            .entities()
            .entity(&self.id)
            .ok_or_else(|| WeaselError::EntityNotFound(self.id.clone()))?;
        // Check position.
        battle
            .space()
            .check_move(PositionClaim::Movement(entity), &self.position)
            .map_err(|err| {
                WeaselError::PositionError(
                    Some(entity.position().clone()),
                    self.position.clone(),
                    Box::new(err),
                )
            })
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Find the entity.
        let entity = battle
            .state
            .entities
            .entity_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: entity {:?} not found", self.id));
        // Take the new position.
        battle.state.space.move_entity(
            PositionClaim::Movement(entity),
            Some(&self.position),
            &mut battle.metrics.write_handle(),
        );
        // Update the entity.
        entity.set_position(self.position.clone());
    }

    fn kind(&self) -> EventKind {
        EventKind::MoveEntity
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `DummyEvent` event.
pub struct MoveEntityTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    position: Position<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for MoveEntityTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `MoveEntity` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(MoveEntity {
            id: self.id.clone(),
            position: self.position.clone(),
        })
    }
}

/// Event to reset the space model.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleController, BattleRules};
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::space::ResetSpace;
/// use weasel::{Server, battle_rules, rules::empty::*};
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// ResetSpace::trigger(&mut server).fire().unwrap();
/// assert_eq!(
///     server.battle().history().events()[0].kind(),
///     EventKind::ResetSpace
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ResetSpace<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<SpaceSeed<R>>: Serialize",
            deserialize = "Option<SpaceSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<SpaceSeed<R>>,
}

impl<R: BattleRules> ResetSpace<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(processor: &mut P) -> ResetSpaceTrigger<R, P> {
        ResetSpaceTrigger {
            processor,
            seed: None,
        }
    }

    /// Returns the new seed.
    pub fn seed(&self) -> &Option<SpaceSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for ResetSpace<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "ResetSpace {{ seed: {:?} }}", self.seed)
    }
}

impl<R: BattleRules> Clone for ResetSpace<R> {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ResetSpace<R> {
    fn verify(&self, _battle: &Battle<R>) -> WeaselResult<(), R> {
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        let rules = &battle.state.space.rules;
        // Generate a new model.
        let mut new_model = rules.generate_model(&self.seed);
        // Translate every entity's position from the old model to the new.
        for entity in battle.state.entities.entities_mut() {
            rules.translate_entity(
                &battle.state.space.model,
                &mut new_model,
                entity,
                event_queue,
                &mut battle.metrics.write_handle(),
            );
        }
        // Set the new model in `Space`.
        battle.state.space.model = new_model;
    }

    fn kind(&self) -> EventKind {
        EventKind::ResetSpace
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ResetSpace` event.
pub struct ResetSpaceTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    seed: Option<SpaceSeed<R>>,
}

impl<'a, R, P> ResetSpaceTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the generation of the new rounds model.
    pub fn seed(&'a mut self, seed: SpaceSeed<R>) -> &'a mut ResetSpaceTrigger<'a, R, P> {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for ResetSpaceTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ResetSpace` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(ResetSpace {
            seed: self.seed.clone(),
        })
    }
}

/// Event to alter the space model.
///
/// Alterations to the space model might have consequences on entities or on other
/// aspects of the battle, as defined in [alter_space](trait.SpaceRules.html#method.alter_space).
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleController, BattleRules};
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::space::AlterSpace;
/// use weasel::{Server, battle_rules, rules::empty::*};
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let alteration = ();
/// AlterSpace::trigger(&mut server, alteration).fire().unwrap();
/// assert_eq!(
///     server.battle().history().events()[0].kind(),
///     EventKind::AlterSpace
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct AlterSpace<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = " SpaceAlteration<R>: Serialize",
            deserialize = " SpaceAlteration<R>: Deserialize<'de>"
        ))
    )]
    alteration: SpaceAlteration<R>,
}

impl<R: BattleRules> AlterSpace<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        alteration: SpaceAlteration<R>,
    ) -> AlterSpaceTrigger<R, P> {
        AlterSpaceTrigger {
            processor,
            alteration,
        }
    }

    /// Returns the alteration to be applied to the space model.
    pub fn alteration(&self) -> &SpaceAlteration<R> {
        &self.alteration
    }
}

impl<R: BattleRules> Debug for AlterSpace<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "AlterSpace {{ alteration: {:?} }}", self.alteration)
    }
}

impl<R: BattleRules> Clone for AlterSpace<R> {
    fn clone(&self) -> Self {
        Self {
            alteration: self.alteration.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for AlterSpace<R> {
    fn verify(&self, _battle: &Battle<R>) -> WeaselResult<(), R> {
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        let rules = &battle.state.space.rules;
        // Apply the alteration.
        rules.alter_space(
            &battle.state.entities,
            &battle.state.rounds,
            &mut battle.state.space.model,
            &self.alteration,
            event_queue,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::AlterSpace
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `AlterSpace` event.
pub struct AlterSpaceTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    alteration: SpaceAlteration<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for AlterSpaceTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `AlterSpace` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(AlterSpace {
            alteration: self.alteration.clone(),
        })
    }
}
