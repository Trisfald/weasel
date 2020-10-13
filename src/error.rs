//! Error and Result module.

use crate::ability::AbilityId;
use crate::battle::{BattleRules, Version};
use crate::creature::CreatureId;
use crate::entity::EntityId;
use crate::event::{DefaultOutput, Event, EventId, EventSinkId};
use crate::metric::MetricIdType;
use crate::object::ObjectId;
use crate::player::PlayerId;
use crate::power::PowerId;
use crate::space::Position;
use crate::status::StatusId;
use crate::team::TeamId;
use std::ops::Range;
use std::result::Result;
use std::{fmt, fmt::Debug};

/// `WeaselError` alias parameterized on the `BattleRules` R.
pub type WeaselErrorType<R> = WeaselError<
    Version<R>,
    TeamId<R>,
    EntityId<R>,
    CreatureId<R>,
    ObjectId<R>,
    Position<R>,
    AbilityId<R>,
    PowerId<R>,
    StatusId<R>,
    MetricIdType<R>,
    Box<dyn Event<R> + Send>,
>;

/// Alias for a `Result` returning a `WeaselError`.
pub type WeaselResult<T, R> = Result<T, WeaselErrorType<R>>;

/// Error type for all kind of errors generated by weasel.
#[derive(Debug, Clone, PartialEq)]
pub enum WeaselError<V, TI, EI, CI, OI, PI, AI, WI, SI, MI, E> {
    /// A generic error.
    GenericError,
    /// Duplicated creature id.
    DuplicatedCreature(CI),
    /// Duplicated object id.
    DuplicatedObject(OI),
    /// Duplicated team id.
    DuplicatedTeam(TI),
    /// The team doesn't exist.
    TeamNotFound(TI),
    /// The creature doesn't exist.
    CreatureNotFound(CI),
    /// The object doesn't exist.
    ObjectNotFound(OI),
    /// Creation of creatures is disabled.
    NewCreatureUnaccepted(TI, Box<Self>),
    /// The creature can't be transferred to the team.
    ConvertedCreatureUnaccepted(TI, CI, Box<Self>),
    /// This creature conversion is not valid.
    InvalidCreatureConversion(TI, CI),
    /// The team is not empty.
    TeamNotEmpty(TI),
    /// Position is invalid.
    PositionError(Option<PI>, PI, Box<Self>),
    /// The entity doesn't exist.
    EntityNotFound(EI),
    /// The event id is not contiguous.
    NonContiguousEventId(EventId, EventId),
    /// A turn is already in progress.
    TurnInProgress,
    /// No turn is in progress.
    NoTurnInProgress,
    /// The actor can't start a new turn.
    ActorNotEligible(EI),
    /// The actor can't act at the moment.
    ActorNotReady(EI),
    /// The actor doesn't know such ability.
    AbilityNotKnown(EI, AI),
    /// The ability can't be activated.
    AbilityNotActivable(EI, AI, Box<Self>),
    /// The team can't act at the moment.
    TeamNotReady(TI),
    /// The team doesn't possess such power.
    PowerNotKnown(TI, WI),
    /// The power can't be invoked.
    PowerNotInvocable(TI, WI, Box<Self>),
    /// Status not present on a character.
    StatusNotPresent(EI, SI),
    /// The event processor is not valid.
    EmptyEventProcessor,
    /// The entity is not a character.
    NotACharacter(EI),
    /// The entity is not an actor.
    NotAnActor(EI),
    /// The entity is not a creature.
    NotACreature(EI),
    /// The entity is not an object.
    NotAnObject(EI),
    /// Attempt to set `Relation::Kin`.
    KinshipRelation,
    /// Attempt to set relation towards oneself.
    SelfRelation,
    /// Two versions of the battle rules are incompatible.
    IncompatibleVersions(V, V),
    /// The battle has already ended.
    BattleEnded,
    /// The metric's type is not correct.
    WrongMetricType(MI),
    /// The `EventPrototype`'s condition is not satisfied.
    ConditionUnsatisfied,
    /// Duplicated event sink id.
    DuplicatedEventSink(EventSinkId),
    /// The event range is invalid.
    InvalidEventRange(Range<EventId>, EventId),
    /// The event sink doesn't exist.
    EventSinkNotFound(EventSinkId),
    /// The player can't fire the event.
    AuthenticationError(Option<PlayerId>, TI),
    /// No authentication in the event.
    MissingAuthentication,
    /// Event can be fired only be the server.
    ServerOnlyEvent,
    /// Failure while packing an user event into a `UserEventPacker`.
    UserEventPackingError(E, String),
    /// Failure while unpacking a `UserEventPacker` into an user event.
    UserEventUnpackingError(String),
    /// The event is invalid.
    InvalidEvent(E, Box<Self>),
    /// An error containing multiple inner errors.
    MultiError(Vec<Self>),
    /// An user defined error.
    UserError(String),
    /// A generic event sink error.
    EventSinkError(String),
}

impl<V, TI, EI, CI, OI, PI, AI, WI, SI, MI, E> fmt::Display
    for WeaselError<V, TI, EI, CI, OI, PI, AI, WI, SI, MI, E>
where
    V: Debug,
    TI: Debug,
    EI: Debug,
    CI: Debug,
    OI: Debug,
    PI: Debug,
    AI: Debug,
    WI: Debug,
    SI: Debug,
    MI: Debug,
    E: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use WeaselError::*;
        match self {
            GenericError => write!(f, "generic error"),
            DuplicatedCreature(id) => write!(f, "duplicated creature with id {:?}", id),
            DuplicatedObject(id) => write!(f, "duplicated object with id {:?}", id),
            DuplicatedTeam(id) => write!(f, "duplicated team with id {:?}", id),
            TeamNotFound(id) => write!(f, "team {:?} not found", id),
            CreatureNotFound(id) => write!(f, "creature {:?} not found", id),
            ObjectNotFound(id) => write!(f, "object {:?} not found", id),
            NewCreatureUnaccepted(id, error) => write!(
                f,
                "team {:?} does not accept new creatures due to {:?}",
                id, error
            ),
            ConvertedCreatureUnaccepted(team_id, creature_id, error) => write!(
                f,
                "team {:?} does not welcome the creature {:?} due to {:?}",
                team_id, creature_id, error
            ),
            InvalidCreatureConversion(team_id, creature_id) => write!(
                f,
                "creature {:?} is already part of team {:?}",
                creature_id, team_id
            ),
            TeamNotEmpty(id) => write!(f, "team {:?} has at least one creature", id),
            PositionError(source, destination, error) => write!(
                f,
                "can't move entity from position {:?} to position {:?} due to {:?}",
                source, destination, error
            ),
            EntityNotFound(id) => write!(f, "entity {:?} not found", id),
            NonContiguousEventId(id, expected) => {
                write!(f, "event has id {:?}, expected {:?}", id, expected)
            }
            TurnInProgress => write!(f, "a turn is already in progress"),
            NoTurnInProgress => write!(f, "no turn is in progress"),
            ActorNotEligible(id) => write!(f, "actor {:?} is not eligible to start a new turn", id),
            ActorNotReady(id) => write!(f, "actor {:?} can't act outside of his turn", id),
            AbilityNotKnown(actor_id, ability_id) => write!(
                f,
                "actor {:?} doesn't know ability {:?}",
                actor_id, ability_id
            ),
            AbilityNotActivable(actor_id, ability_id, error) => write!(
                f,
                "actor {:?} can't activate ability {:?} due to {:?}",
                actor_id, ability_id, error
            ),
            TeamNotReady(id) => write!(f, "team {:?} can't act in this moment", id),
            PowerNotKnown(team_id, power_id) => {
                write!(f, "team {:?} doesn't know power {:?}", team_id, power_id)
            }
            PowerNotInvocable(team_id, power_id, error) => write!(
                f,
                "team {:?} can't invoke power {:?} due to {:?}",
                team_id, power_id, error
            ),
            StatusNotPresent(character_id, status_id) => write!(
                f,
                "character {:?} is not afflicted by status {:?}",
                character_id, status_id
            ),
            NotACharacter(id) => write!(f, "entity {:?} is not a character", id),
            NotAnActor(id) => write!(f, "entity {:?} is not an actor", id),
            NotACreature(id) => write!(f, "entity {:?} is not a creature", id),
            NotAnObject(id) => write!(f, "entity {:?} is not an object", id),
            EmptyEventProcessor => write!(f, "() is not a valid event processor to process events"),
            KinshipRelation => write!(f, "kinship relation can't be explicitly set"),
            SelfRelation => write!(f, "a team can't explicitly set a relation towards itself"),
            IncompatibleVersions(client, server) => write!(
                f,
                "client version {:?} is different from server version {:?}",
                client, server
            ),
            BattleEnded => write!(f, "the battle has ended"),
            WrongMetricType(id) => write!(
                f,
                "metric {:?} exists already with a different counter type",
                id
            ),
            ConditionUnsatisfied => write!(
                f,
                "the condition to apply this event prototype is not satisfied"
            ),
            DuplicatedEventSink(id) => write!(f, "duplicated event sink with id {:?}", id),
            InvalidEventRange(range, history_len) => write!(
                f,
                "event history (0..{}) doesn't contain the event range {:?}",
                history_len, range
            ),
            EventSinkNotFound(id) => write!(f, "event sink {:?} not found", id),
            AuthenticationError(player, team) => write!(
                f,
                "player {:?} doesn't have control over team {:?}",
                player, team
            ),
            MissingAuthentication => write!(f, "event is not linked to any player"),
            ServerOnlyEvent => write!(f, "event can be fired only by the server"),
            UserEventPackingError(event, error) => {
                write!(f, "failed to pack user event {:?}: {}", event, error)
            }
            UserEventUnpackingError(error) => write!(f, "failed to unpack user event: {}", error),
            InvalidEvent(event, error) => write!(f, "{:?} failed due to {:?}, ", event, error),
            MultiError(v) => {
                write!(f, "[")?;
                for err in v {
                    write!(f, "{:?}, ", err)?;
                }
                write!(f, "]")
            }
            UserError(msg) => write!(f, "user error: {}", msg),
            EventSinkError(msg) => write!(f, "sink error: {}", msg),
        }
    }
}

impl<V, TI, EI, CI, OI, PI, AI, WI, SI, MI, E>
    WeaselError<V, TI, EI, CI, OI, PI, AI, WI, SI, MI, E>
{
    /// Unfolds an error, return the inner one in case the original is an `InvalidEvent`.
    /// If not, it returns the original.\
    /// In the case of `MultiError`, unfolds all contained errors.
    ///
    /// # Examples
    /// ```
    /// use weasel::{
    ///     battle_rules, error::WeaselErrorType, event::DummyEvent, rules::empty::*, BattleRules,
    ///     EventTrigger, WeaselError,
    /// };
    ///
    /// battle_rules! {}
    /// let mut processor = ();
    /// let trigger = DummyEvent::trigger(&mut processor);
    /// let error: WeaselErrorType<CustomRules> =
    ///     WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::EmptyEventProcessor));
    /// assert_eq!(error.unfold(), WeaselError::EmptyEventProcessor);
    /// ```
    pub fn unfold(self) -> Self {
        match self {
            Self::InvalidEvent(_, inner) => inner.unfold(),
            Self::MultiError(v) => {
                Self::MultiError(v.into_iter().map(|err| err.unfold()).collect())
            }
            _ => self,
        }
    }

    /// Consumes this error and filters it with the given `filter` function.
    ///
    /// `filter` is applied to this error, to the error inside `InvalidEvent`
    /// and to all errors contained by `MultiError`.\
    /// Only the errors for which `filter` returns true are kept.
    ///
    /// # Examples
    /// ```
    /// use weasel::{
    ///     battle_rules, error::WeaselErrorType, event::DummyEvent, rules::empty::*, BattleRules,
    ///     EventTrigger, WeaselError,
    /// };
    ///
    /// battle_rules! {}
    /// let mut processor = ();
    /// let trigger = DummyEvent::trigger(&mut processor);
    /// let error: WeaselErrorType<CustomRules> =
    ///     WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::EmptyEventProcessor));
    /// assert_eq!(
    ///     error
    ///         .filter(|err| {
    ///             if let WeaselError::EmptyEventProcessor = err {
    ///                 false
    ///             } else {
    ///                 true
    ///             }
    ///         })
    ///         .err(),
    ///     None
    /// );
    /// ```
    pub fn filter<F>(self, op: F) -> Result<(), Self>
    where
        F: Fn(&Self) -> bool + Copy,
    {
        if !op(&self) {
            Ok(())
        } else {
            match self {
                Self::InvalidEvent(event, error) => {
                    let new_error = error.filter(op);
                    if new_error.is_err() {
                        Err(Self::InvalidEvent(
                            event,
                            Box::new(new_error.err().unwrap()),
                        ))
                    } else {
                        Ok(())
                    }
                }
                Self::MultiError(v) => {
                    let mut new_errors = Vec::new();
                    for error in v {
                        let new_error = error.filter(op);
                        if new_error.is_err() {
                            new_errors.push(new_error.err().unwrap());
                        }
                    }
                    if new_errors.is_empty() {
                        Ok(())
                    } else if new_errors.len() == 1 {
                        Err(new_errors.pop().unwrap())
                    } else {
                        Err(Self::MultiError(new_errors))
                    }
                }
                _ => Err(self),
            }
        }
    }
}

impl<R> DefaultOutput<R> for WeaselResult<(), R>
where
    R: BattleRules,
{
    type Error = WeaselErrorType<R>;

    fn ok() -> Self {
        Ok(())
    }

    fn err(self) -> Option<Self::Error> {
        self.err()
    }

    fn result(self) -> WeaselResult<(), R> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleRules;
    use crate::event::{DummyEvent, EventTrigger};
    use crate::{battle_rules, rules::empty::*};

    #[test]
    #[allow(clippy::let_unit_value)]
    fn unfold() {
        battle_rules! {}
        let mut processor = ();
        let trigger = DummyEvent::trigger(&mut processor);
        // Test a direct unfolding.
        let error: WeaselErrorType<CustomRules> =
            WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::EmptyEventProcessor));
        assert_eq!(error.clone().unfold(), WeaselError::EmptyEventProcessor);
        // Test unfolding of multierrors.
        let error: WeaselErrorType<CustomRules> = WeaselError::MultiError(vec![error]);
        assert_eq!(
            error.unfold(),
            WeaselError::MultiError(vec![WeaselError::EmptyEventProcessor])
        );
    }

    #[test]
    #[allow(clippy::let_unit_value)]
    fn filter() {
        battle_rules! {}
        let mut processor = ();
        let trigger = DummyEvent::trigger(&mut (processor));
        let filter_fn = |err: &WeaselErrorType<_>| !matches!(err, WeaselError::EmptyEventProcessor);
        // Test a direct filtering.
        let error_to_filter: WeaselErrorType<CustomRules> =
            WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::EmptyEventProcessor));
        assert_eq!(error_to_filter.clone().filter(filter_fn).err(), None);
        // Test filtering of multierrors.
        let error: WeaselErrorType<CustomRules> =
            WeaselError::MultiError(vec![error_to_filter.clone()]);
        assert_eq!(error.filter(filter_fn).err(), None);
        // Filter only the right errors.
        let error: WeaselErrorType<CustomRules> =
            WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::TurnInProgress));
        let error: WeaselErrorType<CustomRules> =
            WeaselError::MultiError(vec![error_to_filter, error]);
        assert_eq!(
            error.filter(filter_fn).err(),
            Some(WeaselError::InvalidEvent(
                trigger.event(),
                Box::new(WeaselError::TurnInProgress)
            ))
        );
    }
}
