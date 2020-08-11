//! Module to manage abilities.

use crate::actor::{Action, ActorRules};
use crate::battle::{Battle, BattleRules};
use crate::entity::EntityId;
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventRights, EventTrigger};
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Type to represent an ability.
///
/// Abilities are actions that actors can undertake in order to modify an aspect of themselves or
/// of the world. Typical abilities are movements and attacks.
pub type Ability<R> = <<R as BattleRules>::AR as ActorRules<R>>::Ability;

/// Alias for `Ability<R>::Id`.
pub type AbilityId<R> = <Ability<R> as Id>::Id;

/// Type to drive the generation of a given actor's set of abilities.
pub type AbilitiesSeed<R> = <<R as BattleRules>::AR as ActorRules<R>>::AbilitiesSeed;

/// Type to customize in which way an ability is activated.
///
/// For example, this's useful in case you have abilities which can be activated with a
/// different degree of intensity.
pub type Activation<R> = <<R as BattleRules>::AR as ActorRules<R>>::Activation;

/// Encapsulatess the data used to describe an alteration of one or more abilities.
pub type AbilitiesAlteration<R> = <<R as BattleRules>::AR as ActorRules<R>>::AbilitiesAlteration;

/// Event to make an actor activate an ability.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, ActivateAbility, Battle, BattleRules, CreateCreature,
///     CreateTeam, EntityId, EventTrigger, Server, StartRound,
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
/// StartRound::trigger(&mut server, EntityId::Creature(creature_id))
///     .fire()
///     .unwrap();
///
/// let ability_id = 99;
/// let result =
///     ActivateAbility::trigger(&mut server, EntityId::Creature(creature_id), ability_id)
///         .fire();
/// // We get an error because the creature doesn't know this ability.
/// // The set of abilities known by creatures must defined in 'ActorRules'.
/// assert!(result.is_err());
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ActivateAbility<R: BattleRules> {
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
            serialize = "AbilityId<R>: Serialize",
            deserialize = "AbilityId<R>: Deserialize<'de>"
        ))
    )]
    ability_id: AbilityId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<Activation<R>>: Serialize",
            deserialize = "Option<Activation<R>>: Deserialize<'de>"
        ))
    )]
    activation: Option<Activation<R>>,
}

impl<R: BattleRules> ActivateAbility<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        entity_id: EntityId<R>,
        ability_id: AbilityId<R>,
    ) -> ActivateAbilityTrigger<R, P> {
        ActivateAbilityTrigger {
            processor,
            entity_id,
            ability_id,
            activation: None,
        }
    }

    /// Returns the id of the actor who is activating the ability.
    pub fn entity_id(&self) -> &EntityId<R> {
        &self.entity_id
    }

    /// Returns the id of the ability to be activated.
    pub fn ability_id(&self) -> &AbilityId<R> {
        &self.ability_id
    }

    /// Returns the activation profile for the ability.
    pub fn activation(&self) -> &Option<Activation<R>> {
        &self.activation
    }
}

impl<R: BattleRules> std::fmt::Debug for ActivateAbility<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActivateAbility {{ entity_id: {:?}, ability_id: {:?}, activation: {:?} }}",
            self.entity_id, self.ability_id, self.activation
        )
    }
}

impl<R: BattleRules> Clone for ActivateAbility<R> {
    fn clone(&self) -> Self {
        ActivateAbility {
            entity_id: self.entity_id.clone(),
            ability_id: self.ability_id.clone(),
            activation: self.activation.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ActivateAbility<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Check if this entity can is an actor.
        if !self.entity_id.is_actor() {
            return Err(WeaselError::NotAnActor(self.entity_id.clone()));
        }
        // Verify that the actor exists.
        if let Some(actor) = battle.entities().actor(&self.entity_id) {
            // Verify that the actor can act.
            if !battle.state.rounds.is_acting(&self.entity_id) {
                return Err(WeaselError::ActorNotReady(self.entity_id.clone()));
            }
            // Verify if the creature knowns this ability.
            if let Some(ability) = actor.ability(&self.ability_id) {
                // Verify if this ability can be activated.
                battle
                    .rules
                    .actor_rules()
                    .activable(&battle.state, Action::new(actor, ability, &self.activation))
                    .map_err(|err| {
                        WeaselError::AbilityNotActivable(
                            self.entity_id.clone(),
                            self.ability_id.clone(),
                            Box::new(err),
                        )
                    })
            } else {
                Err(WeaselError::AbilityNotKnown(
                    self.entity_id.clone(),
                    self.ability_id.clone(),
                ))
            }
        } else {
            Err(WeaselError::EntityNotFound(self.entity_id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        let actor = battle
            .state
            .entities
            .actor(&self.entity_id)
            .unwrap_or_else(|| {
                panic!("constraint violated: entity {:?} not found", self.entity_id)
            });
        let ability = actor.ability(&self.ability_id).unwrap_or_else(|| {
            panic!(
                "constraint violated: ability {:?} not found in actor {:?}",
                self.ability_id, self.entity_id
            )
        });
        battle.rules.actor_rules().activate(
            &battle.state,
            Action::new(actor, ability, &self.activation),
            event_queue,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::ActivateAbility
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn rights<'a>(&'a self, battle: &'a Battle<R>) -> EventRights<'a, R> {
        let actor = battle
            .state
            .entities
            .actor(&self.entity_id)
            .unwrap_or_else(|| {
                panic!("constraint violated: entity {:?} not found", self.entity_id)
            });
        EventRights::Team(actor.team_id())
    }
}

/// Trigger to build and fire an `ActivateAbility` event.
pub struct ActivateAbilityTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    entity_id: EntityId<R>,
    ability_id: AbilityId<R>,
    activation: Option<Activation<R>>,
}

impl<'a, R, P> ActivateAbilityTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds an activation profile to customize this ability instance.
    pub fn activation(&'a mut self, activation: Activation<R>) -> &'a mut Self {
        self.activation = Some(activation);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for ActivateAbilityTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `ActivateAbility` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(ActivateAbility {
            entity_id: self.entity_id.clone(),
            ability_id: self.ability_id.clone(),
            activation: self.activation.clone(),
        })
    }
}
