//! Error and Result module.

use crate::ability::AbilityId;
use crate::battle::{BattleRules, Version};
use crate::creature::CreatureId;
use crate::entity::EntityId;
use crate::event::{DefaultOutput, Event, EventId, EventSinkId};
use crate::metric::MetricIdType;
use crate::player::PlayerId;
use crate::space::Position;
use crate::team::TeamId;
use std::error;
use std::ops::Range;
use std::result::Result;
use std::{fmt, fmt::Debug};

/// `WeaselError` alias parameterized on the `BattleRules` R.
pub type WeaselErrorType<R> = WeaselError<
    Version<R>,
    TeamId<R>,
    EntityId<R>,
    CreatureId<R>,
    Position<R>,
    AbilityId<R>,
    MetricIdType<R>,
    Box<dyn Event<R>>,
>;

/// Alias for a `Result` returning a `WeaselError`.
pub type WeaselResult<T, R> = Result<T, WeaselErrorType<R>>;

/// Error type for all kind of errors generated by weasel.
#[derive(Debug, Clone, PartialEq)]
pub enum WeaselError<V, TI, EI, CI, PI, AI, MI, E> {
    /// Duplicated creature id.
    DuplicatedCreature(CI),
    /// Duplicated team id.
    DuplicatedTeam(TI),
    /// The team doesn't exist.
    TeamNotFound(TI),
    /// The creature doesn't exist.
    CreatureNotFound(CI),
    /// Creation of creatures is disabled.
    NewCreatureUnaccepted(TI),
    /// The creature can't be transferred to the team.
    ConvertedCreatureUnaccepted(TI, CI),
    /// This creature conversion is not valid.
    InvalidCreatureConversion(TI, CI),
    /// Position is invalid.
    PositionError(Option<PI>, PI),
    /// The entity doesn't exist.
    EntityNotFound(EI),
    /// The event id is not contiguous.
    NonContiguousEventId(EventId, EventId),
    /// A round is already in progress.
    RoundInProgress,
    /// No round is in progress.
    NoRoundInProgress,
    /// The actor can't start a new round.
    ActorNotEligible(EI),
    /// The actor can't act at the moment.
    ActorNotReady(EI),
    /// Actor does not know such ability.
    AbilityNotKnown(EI, AI),
    /// The ability can't be activated.
    AbilityNotActivable(EI, AI),
    /// The event processor is not valid.
    EmptyEventProcessor,
    /// The entity is not a character.
    NotACharacter(EI),
    /// The entity is not an actor.
    NotAnActor(EI),
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
    #[allow(clippy::type_complexity)]
    InvalidEvent(E, Box<WeaselError<V, TI, EI, CI, PI, AI, MI, E>>),
    /// An error containing multiple inner errors.
    #[allow(clippy::type_complexity)]
    MultiError(Vec<WeaselError<V, TI, EI, CI, PI, AI, MI, E>>),
    /// An user defined error.
    UserError(String),
    /// A generic event sink error.
    EventSinkError(String),
}

impl<V, TI, EI, CI, PI, AI, MI, E> fmt::Display for WeaselError<V, TI, EI, CI, PI, AI, MI, E>
where
    V: Debug,
    TI: Debug,
    EI: Debug,
    CI: Debug,
    PI: Debug,
    AI: Debug,
    MI: Debug,
    E: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WeaselError::DuplicatedCreature(id) => {
                write!(f, "duplicated creature with id {:?}", id)
            }
            WeaselError::DuplicatedTeam(id) => write!(f, "duplicated team with id {:?}", id),
            WeaselError::TeamNotFound(id) => write!(f, "team {:?} not found", id),
            WeaselError::CreatureNotFound(id) => write!(f, "creature {:?} not found", id),
            WeaselError::NewCreatureUnaccepted(id) => {
                write!(f, "team {:?} does not accept new creatures", id)
            }
            WeaselError::ConvertedCreatureUnaccepted(team_id, creature_id) => write!(
                f,
                "team {:?} does not welcome the creature {:?}",
                team_id, creature_id
            ),
            WeaselError::InvalidCreatureConversion(team_id, creature_id) => write!(
                f,
                "creature {:?} is already part of team {:?}",
                creature_id, team_id
            ),
            WeaselError::PositionError(source, destination) => write!(
                f,
                "can't move entity from position {:?} to position {:?}",
                source, destination
            ),
            WeaselError::EntityNotFound(id) => write!(f, "entity {:?} not found", id),
            WeaselError::NonContiguousEventId(id, expected) => {
                write!(f, "event has id {:?}, expected {:?}", id, expected)
            }
            WeaselError::RoundInProgress => write!(f, "a round is already in progress"),
            WeaselError::NoRoundInProgress => write!(f, "no round is in progress"),
            WeaselError::ActorNotEligible(id) => {
                write!(f, "actor {:?} is not eligible to start a new round", id)
            }
            WeaselError::ActorNotReady(id) => {
                write!(f, "actor {:?} can't act outside of his round", id)
            }
            WeaselError::AbilityNotKnown(actor_id, ability_id) => write!(
                f,
                "actor {:?} doesn't known ability {:?}",
                actor_id, ability_id
            ),
            WeaselError::AbilityNotActivable(actor_id, ability_id) => write!(
                f,
                "actor {:?} can't activate ability {:?}",
                actor_id, ability_id
            ),
            WeaselError::NotACharacter(id) => write!(f, "entity {:?} is not a character", id),
            WeaselError::NotAnActor(id) => write!(f, "entity {:?} is not an actor", id),
            WeaselError::EmptyEventProcessor => {
                write!(f, "() is not a valid event processor to process events")
            }
            WeaselError::KinshipRelation => write!(f, "kinship relation can't be explicitly set"),
            WeaselError::SelfRelation => {
                write!(f, "a team can't explicitly set a relation towards itself")
            }
            WeaselError::IncompatibleVersions(client, server) => write!(
                f,
                "client version {:?} is different from server version {:?}",
                client, server
            ),
            WeaselError::BattleEnded => write!(f, "the battle has ended"),
            WeaselError::WrongMetricType(id) => write!(
                f,
                "metric {:?} exists already with a different counter type",
                id
            ),
            WeaselError::ConditionUnsatisfied => write!(
                f,
                "the condition to apply this event prototype is not satisfied"
            ),
            WeaselError::DuplicatedEventSink(id) => {
                write!(f, "duplicated event sink with id {:?}", id)
            }
            WeaselError::InvalidEventRange(range, history_len) => write!(
                f,
                "event history (0..{}) doesn't contain the event range {:?}",
                history_len, range
            ),
            WeaselError::EventSinkNotFound(id) => write!(f, "event sink {:?} not found", id),
            WeaselError::AuthenticationError(player, team) => write!(
                f,
                "player {:?} doesn't have control over team {:?}",
                player, team
            ),
            WeaselError::MissingAuthentication => write!(f, "event is not linked to any player"),
            WeaselError::ServerOnlyEvent => write!(f, "event can be fired only by the server"),
            WeaselError::UserEventPackingError(event, error) => {
                write!(f, "failed to pack user event {:?}: {}", event, error)
            }
            WeaselError::UserEventUnpackingError(error) => {
                write!(f, "failed to unpack user event: {}", error)
            }
            WeaselError::InvalidEvent(event, error) => {
                write!(f, "{:?} failed due to {:?}, ", event, error)
            }
            WeaselError::MultiError(v) => {
                write!(f, "[")?;
                for err in v {
                    write!(f, "{:?}, ", err)?;
                }
                write!(f, "]")
            }
            WeaselError::UserError(msg) => write!(f, "user error: {}", msg),
            WeaselError::EventSinkError(msg) => write!(f, "sink error: {}", msg),
        }
    }
}

impl<V, TI, EI, CI, PI, AI, MI, E> error::Error for WeaselError<V, TI, EI, CI, PI, AI, MI, E>
where
    V: Debug,
    TI: Debug,
    EI: Debug,
    CI: Debug,
    PI: Debug,
    AI: Debug,
    MI: Debug,
    E: Debug,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl<V, TI, EI, CI, PI, AI, MI, E> WeaselError<V, TI, EI, CI, PI, AI, MI, E> {
    /// Unfolds an error, return the inner one in case the original is an `InvalidEvent`.
    /// If not, it returns the original.\
    /// In the case of `MultiError`, unfolds all contained errors.
    ///
    /// # Examples
    /// ```
    /// use weasel::event::{EventTrigger, DummyEvent};
    /// use weasel::error::{WeaselErrorType, WeaselError};
    /// use weasel::battle::BattleRules;
    /// use weasel::{battle_rules, rules::empty::*};
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
            WeaselError::InvalidEvent(_, inner) => inner.unfold(),
            WeaselError::MultiError(v) => {
                WeaselError::MultiError(v.into_iter().map(|err| err.unfold()).collect())
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
    /// use weasel::event::{EventTrigger, DummyEvent};
    /// use weasel::error::{WeaselErrorType, WeaselError};
    /// use weasel::battle::BattleRules;
    /// use weasel::{battle_rules, rules::empty::*};
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
                WeaselError::InvalidEvent(event, error) => {
                    let new_error = error.filter(op);
                    if new_error.is_err() {
                        Err(WeaselError::InvalidEvent(
                            event,
                            Box::new(new_error.err().unwrap()),
                        ))
                    } else {
                        Ok(())
                    }
                }
                WeaselError::MultiError(v) => {
                    let mut new_errors = Vec::new();
                    for error in v {
                        let new_error = error.filter(op);
                        if new_error.is_err() {
                            new_errors.push(new_error.err().unwrap());
                        }
                    }
                    if new_errors.is_empty() {
                        Ok(())
                    } else {
                        Err(WeaselError::MultiError(new_errors))
                    }
                }
                _ => Err(self),
            }
        }
    }
}

impl<R> DefaultOutput for WeaselResult<(), R>
where
    R: BattleRules,
{
    type Error = WeaselErrorType<R>;

    fn ok() -> WeaselResult<(), R> {
        Ok(())
    }

    fn err(self) -> Option<Self::Error> {
        self.err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleRules;
    use crate::event::{DummyEvent, EventTrigger};
    use crate::{battle_rules, rules::empty::*};

    #[test]
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
    fn filter() {
        battle_rules! {}
        let mut processor = ();
        let trigger = DummyEvent::trigger(&mut processor);
        // Test a direct filtering.
        let error: WeaselErrorType<CustomRules> =
            WeaselError::InvalidEvent(trigger.event(), Box::new(WeaselError::EmptyEventProcessor));
        assert_eq!(
            error
                .clone()
                .filter(|err| {
                    if let WeaselError::EmptyEventProcessor = err {
                        false
                    } else {
                        true
                    }
                })
                .err(),
            None
        );
        // Test filtering of multierrors.
        let error: WeaselErrorType<CustomRules> = WeaselError::MultiError(vec![error]);
        assert_eq!(
            error
                .filter(|err| {
                    if let WeaselError::EmptyEventProcessor = err {
                        false
                    } else {
                        true
                    }
                })
                .err(),
            None
        );
    }
}