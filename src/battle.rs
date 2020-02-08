//! Battle module.

use crate::actor::ActorRules;
use crate::character::CharacterRules;
use crate::entity::Entities;
use crate::entropy::{Entropy, EntropyRules};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{
    ClientEventPrototype, Event, EventKind, EventProcessor, EventPrototype, EventQueue,
    EventTrigger, EventWrapper, Prioritized, VersionedEventWrapper,
};
use crate::fight::FightRules;
use crate::history::History;
use crate::metric::{Metrics, ReadMetrics};
use crate::player::{Rights, RightsHandle, RightsHandleMut};
use crate::round::{Rounds, RoundsRules};
use crate::space::{Space, SpaceRules};
use crate::team::{ConcludeObjectives, TeamId, TeamRules};
use crate::user::UserRules;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Range;

/// Type to define a callback invoked each time an event is processed.
///
/// `BattleState` is a snapshot of the state of the battle,
/// taken just after the event has been applied.\
/// `EventQueue` is an event processor that can be used to fire events.
pub type EventCallback<R> =
    Box<dyn FnMut(&EventWrapper<R>, &BattleState<R>, &mut Option<EventQueue<R>>)>;

/// Represent the in-game world from the point of view of the tactical combat system.
///
/// Battle is the core object in weasel, since it contains all entities, teams, the
/// event timeline and all other modules necessary to have a representation of the world.
pub struct Battle<R: BattleRules> {
    pub(crate) state: BattleState<R>,
    pub(crate) entropy: Entropy<R>,
    history: History<R>,
    pub(crate) rules: R,
    pub(crate) event_callback: Option<EventCallback<R>>,
    pub(crate) metrics: Metrics<R>,
    rights: Rights<R>,
}

impl<R: BattleRules + 'static> Battle<R> {
    /// Returns a battle builder.
    pub fn builder(rules: R) -> BattleBuilder<R> {
        BattleBuilder {
            rules,
            event_callback: None,
        }
    }

    /// Verifies the consistency of an event.
    #[allow(clippy::borrowed_box)]
    pub(crate) fn verify_event(&self, event: &Box<dyn Event<R>>) -> WeaselResult<(), R> {
        if self.phase() == BattlePhase::Ended {
            Err(WeaselError::BattleEnded)
        } else {
            event.verify(&self)
        }
    }

    /// Verifies the consistency of an `EventPrototype`.
    pub(crate) fn verify_prototype(&self, event: &EventPrototype<R>) -> WeaselResult<(), R> {
        // Verify condition.
        if let Some(condition) = event.condition() {
            if !condition(&self.state) {
                return Err(WeaselError::ConditionUnsatisfied);
            }
        }
        // Verify event.
        self.verify_event(event)
    }

    /// Verifies the consistency of a `VersionedEventWrapper`.
    pub(crate) fn verify_wrapper(&self, event: &VersionedEventWrapper<R>) -> WeaselResult<(), R> {
        // Verify version.
        let version = self.rules.version();
        if event.version() != version {
            return Err(WeaselError::IncompatibleVersions(
                version.clone(),
                event.version().clone(),
            ));
        }
        // Verify timeline consistency.
        self.history.verify_event(event.wrapper())?;
        // Verify event.
        self.verify_event(event.wrapper())
    }

    pub(crate) fn verify_client(&self, event: &ClientEventPrototype<R>) -> WeaselResult<(), R> {
        // Verify version.
        let version = self.rules.version();
        if event.version() != version {
            return Err(WeaselError::IncompatibleVersions(
                event.version().clone(),
                version.clone(),
            ));
        }
        // Verify event.
        self.verify_event(event)
    }

    /// Promotes an `EventPrototype` into an `EventWrapper`.
    pub(crate) fn promote(&self, event: EventPrototype<R>) -> EventWrapper<R> {
        event.promote(self.history.next_id())
    }

    /// Apply an event to the world.
    /// Takes in a optional `EventQueue`, to eventually store new prototypes derived from `event`.
    pub(crate) fn apply(&mut self, event: &EventWrapper<R>, queue: &mut Option<EventQueue<R>>) {
        // Apply the event to the world.
        event.apply(self, queue);
        // Save into history.
        self.history.archive(event);
        // Check teams' objectives.
        Battle::check_objectives(
            &self.state,
            &self.rules.team_rules(),
            &self.metrics.read_handle(),
            &mut queue.as_mut().map(|queue| Prioritized::new(queue)),
            Checkpoint::EventEnd,
        );
        // Invoke user callback.
        if let Some(cb) = &mut self.event_callback {
            cb(event, &self.state, queue);
        }
    }

    /// Ends the battle.
    pub(crate) fn end(&mut self) {
        self.state.phase = BattlePhase::Ended;
    }

    /// Returns in which phase is the battle.
    pub fn phase(&self) -> BattlePhase {
        self.state.phase
    }

    /// Returns the entities manager for this battle.
    pub fn entities(&self) -> &Entities<R> {
        &self.state.entities
    }

    /// Returns the history of this battle.
    pub fn history(&self) -> &History<R> {
        &self.history
    }

    /// Returns this battle's rules.
    pub fn rules(&self) -> &R {
        &self.rules
    }

    /// Returns this battle's space representation.
    pub fn space(&self) -> &Space<R> {
        &self.state.space
    }

    /// Returns the entropy manager for this battle.
    pub fn entropy(&self) -> &Entropy<R> {
        &self.entropy
    }

    /// Returns the rounds manager for this battle.
    pub fn rounds(&self) -> &Rounds<R> {
        &self.state.rounds
    }

    /// Returns a handle from which metrics can be read.
    pub fn metrics(&self) -> ReadMetrics<R> {
        self.metrics.read_handle()
    }

    /// Returns a handle to access the players' rights to control one or more teams.
    pub(crate) fn rights(&self) -> RightsHandle<R> {
        RightsHandle::new(&self.rights)
    }

    /// Returns a mutable handle to manage the players' rights to control one or more teams.
    pub(crate) fn rights_mut<'a>(
        &'a mut self,
    ) -> RightsHandleMut<R, impl Iterator<Item = &'a TeamId<R>>> {
        RightsHandleMut::new(
            &mut self.rights,
            self.state.entities().teams().map(|team| team.id()),
        )
    }

    /// Returns an iterator over all history events in a range, versioned.
    ///
    /// The range must be valid.
    pub fn versioned_events<'a>(
        &'a self,
        range: Range<usize>,
    ) -> impl Iterator<Item = VersionedEventWrapper<R>> + 'a {
        self.history().events()[range]
            .iter()
            .map(move |e| e.clone().version(self.rules().version().clone()))
    }

    /// Checks if one or more teams have completed their objectives and creates events accordingly.
    pub(crate) fn check_objectives<P>(
        state: &BattleState<R>,
        rules: &R::TR,
        metrics: &ReadMetrics<R>,
        processor: &mut P,
        checkpoint: Checkpoint,
    ) where
        P: EventProcessor<R>,
    {
        /// Put common login into a macro.
        macro_rules! run_check {
            ($function: ident) => {{
                for team in state
                    .entities
                    .teams()
                    .filter(|team| team.conclusion().is_none())
                {
                    if let Some(conclusion) = rules.$function(state, team, metrics) {
                        // Team has a conclusion, fire an event.
                        ConcludeObjectives::trigger(processor, team.id().clone(), conclusion)
                            .fire();
                    }
                    // No changes.
                }
            }};
        }

        match checkpoint {
            Checkpoint::RoundEnd => {
                run_check!(check_objectives_on_round);
            }
            Checkpoint::EventEnd => {
                run_check!(check_objectives_on_event);
            }
        }
    }
}

/// Checkpoint in which a `check_objective` is run.
pub(crate) enum Checkpoint {
    /// At the end of a round.
    RoundEnd,
    /// At the end of an event.
    EventEnd,
}

/// Owns he battle submodules that contain the current state of the battle.
pub struct BattleState<R: BattleRules> {
    pub(crate) entities: Entities<R>,
    pub(crate) space: Space<R>,
    pub(crate) rounds: Rounds<R>,
    pub(crate) phase: BattlePhase,
}

impl<R: BattleRules> BattleState<R> {
    /// Returns the entities manager for this battle.
    pub fn entities(&self) -> &Entities<R> {
        &self.entities
    }

    /// Returns this battle's space representation.
    pub fn space(&self) -> &Space<R> {
        &self.space
    }

    /// Returns the rounds manager for this battle.
    pub fn rounds(&self) -> &Rounds<R> {
        &self.rounds
    }

    /// Returns in which phase is the battle.
    pub fn phase(&self) -> BattlePhase {
        self.phase
    }
}

/// All possible phases in which a battle can be.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BattlePhase {
    /// The battle has started.
    Started,
    /// The battle has ended.
    Ended,
}

/// Contains the set of rules for this battle.
/// It's a trait that uses composition to gather all other subsystem rules in a single place.
///
/// All rules must be deterministic, otherwise the consistency of events is not guaranteed.
pub trait BattleRules: std::marker::Sized {
    /// Type defining the `TeamRules`.
    type TR: TeamRules<Self>;
    /// Type defining the `CharacterRules`.
    type CR: CharacterRules<Self>;
    /// Type defining the `ActorRules`.
    type AR: ActorRules<Self>;
    /// Type defining the `FightRules`.
    type FR: FightRules<Self>;
    /// Type defining the `UserRules`.
    type UR: UserRules<Self>;
    /// Type defining the `SpaceRules`.
    type SR: SpaceRules<Self>;
    /// Type defining the `RoundsRules`.
    type RR: RoundsRules<Self>;
    /// Type defining the `EntropyRules`.
    type ER: EntropyRules;

    #[cfg(not(feature = "serialization"))]
    /// See [Version](type.Version.html).
    type Version: PartialEq + Debug + Clone;
    #[cfg(feature = "serialization")]
    /// See [Version](type.Version.html).
    type Version: PartialEq + Debug + Clone + Serialize + for<'a> Deserialize<'a>;

    /// Returns a reference to the team rules.
    fn team_rules(&self) -> &Self::TR;

    /// Returns a reference to the character rules.
    fn character_rules(&self) -> &Self::CR;

    /// Returns a reference to the actor rules.
    fn actor_rules(&self) -> &Self::AR;

    /// Returns a reference to the fight rules.
    fn fight_rules(&self) -> &Self::FR;

    /// Returns a reference to the user rules.
    fn user_rules(&self) -> &Self::UR;

    /// Consumes and returns the space rules.
    fn space_rules(&mut self) -> Self::SR;

    /// Consumes and returns the rounds rules.
    fn rounds_rules(&mut self) -> Self::RR;

    /// Consumes and returns the entropy rules.
    fn entropy_rules(&mut self) -> Self::ER;

    /// Returns the version of this battle rules.
    fn version(&self) -> &Self::Version;
}

/// Type to represent the version of this battle rules.
/// It is used to verify each event. You can use `()` to disable versioning.
pub type Version<R> = <R as BattleRules>::Version;

/// A builder object to create a battle.
pub struct BattleBuilder<R: BattleRules> {
    rules: R,
    event_callback: Option<EventCallback<R>>,
}

impl<R: BattleRules> BattleBuilder<R> {
    /// Sets an event callback that will be invoked each time an event is applied to the world.
    pub fn event_callback(mut self, event_callback: EventCallback<R>) -> BattleBuilder<R> {
        self.event_callback = Some(event_callback);
        self
    }

    /// Creates a new battle.
    pub fn build(mut self) -> Battle<R> {
        Battle {
            state: BattleState {
                entities: Entities::new(),
                space: Space::new(None, self.rules.space_rules()),
                rounds: Rounds::new(None, self.rules.rounds_rules()),
                phase: BattlePhase::Started,
            },
            entropy: Entropy::new(None, self.rules.entropy_rules()),
            history: History::new(),
            rules: self.rules,
            event_callback: self.event_callback,
            metrics: Metrics::new(),
            rights: Rights::new(),
        }
    }
}

/// Event to end the battle.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct EndBattle<R> {
    #[cfg_attr(feature = "serialization", serde(skip))]
    _phantom: PhantomData<R>,
}

impl<R: BattleRules> EndBattle<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(processor: &mut P) -> EndBattleTrigger<R, P> {
        EndBattleTrigger {
            processor,
            _phantom: PhantomData,
        }
    }
}

impl<R> std::fmt::Debug for EndBattle<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EndBattle {{ }}")
    }
}

impl<R> Clone for EndBattle<R> {
    fn clone(&self) -> Self {
        EndBattle {
            _phantom: PhantomData,
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for EndBattle<R> {
    fn verify(&self, _battle: &Battle<R>) -> WeaselResult<(), R> {
        // Battle can be ended at any moment.
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        battle.end();
    }

    fn kind(&self) -> EventKind {
        EventKind::EndBattle
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `EndBattle` event.
pub struct EndBattleTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    _phantom: PhantomData<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for EndBattleTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `EndBattle` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(EndBattle {
            _phantom: self._phantom,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;
    use crate::server::Server;
    use crate::team::CreateTeam;
    use crate::util::tests::{dummy, team};
    use crate::{battle_rules, rules::empty::*};

    battle_rules! {}

    fn cb(
        event: &EventWrapper<CustomRules>,
        _: &BattleState<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
    ) {
        match event.kind() {
            // Each time a team is created, check the team id and fire a dummy event.
            EventKind::CreateTeam => {
                let create_team: &CreateTeam<CustomRules> =
                    match event.as_any().downcast_ref::<CreateTeam<CustomRules>>() {
                        Some(b) => b,
                        None => panic!("incorrect cast!"),
                    };
                assert_eq!(*create_team.id(), 1);
                dummy(event_queue);
            }
            _ => {} // Do nothing.
        }
    }

    #[test]
    fn event_callback() {
        let battle = Battle::builder(CustomRules::new())
            .event_callback(Box::new(cb))
            .build();
        let mut server = Server::builder(battle).build();
        // Create a team.
        team(&mut server, 1);
        // Check whether or not the dummy event was fired.
        assert_eq!(
            server.battle().history().events()[1].kind(),
            EventKind::DummyEvent
        );
    }
}
