//! Module to handle serialization and deserialization.

use crate::ability::ActivateAbility;
use crate::actor::{AlterAbilities, RegenerateAbilities};
use crate::battle::{BattleRules, EndBattle, Version};
use crate::character::{AlterStatistics, RegenerateStatistics};
use crate::creature::{ConvertCreature, CreateCreature, RemoveCreature};
use crate::entropy::ResetEntropy;
use crate::event::{
    ClientEventPrototype, DummyEvent, Event, EventId, EventKind, EventWrapper,
    VersionedEventWrapper,
};
use crate::fight::ApplyImpact;
use crate::object::{CreateObject, RemoveObject};
use crate::player::PlayerId;
use crate::round::{EndRound, EndTurn, EnvironmentTurn, ResetRounds, StartTurn};
use crate::space::{AlterSpace, MoveEntity, ResetSpace};
use crate::status::{AlterStatuses, ClearStatus, InflictStatus};
use crate::team::{
    AlterPowers, ConcludeObjectives, CreateTeam, RegeneratePowers, RemoveTeam, ResetObjectives,
    SetRelations,
};
use crate::user::{UserEventPackage, UserEventPacker};
use serde::{Deserialize, Serialize};

/// Macro to panic on incorrect cast.
macro_rules! bad_cast {
    () => {{
        panic!("incorrect cast!")
    }};
}

/// Generates the boxed() method for `FlatEvent`.
macro_rules! flat_event_boxed {
    ($( $x:ident ),* $(,)?) => {
        /// Transforms this flattened event into a boxed event trait object.
        pub fn boxed(self) -> Box<dyn Event<R> + Send> {
            // Generate a match with an arm for every concrete event type.
            match self {
                $(FlatEvent::$x(flat) => {
                    Box::new(flat) as Box<dyn Event<R> + Send>
                })*
                FlatEvent::UserEventPackage(packer) => {
                    packer.boxed().unwrap_or_else(|err| {
                        panic!("{:?}", err)
                    })
                }
            }
        }
    }
}

/// Generates the flattened() method for `FlatEvent`.
macro_rules! flat_event_flattened {
    ($( $x:ident ),* $(,)?) => {
        /// Transforms a boxed event trait object into a flattened event.
        pub fn flattened(event: Box<dyn Event<R> + Send>) -> FlatEvent<R> {
            // Generate a match with an arm for every concrete event type.
            match event.kind() {
                $(EventKind::$x => {
                    match event.as_any().downcast_ref::<$x<R>>() {
                        Some(event) => {
                            FlatEvent::$x(event.clone())
                        },
                        None => bad_cast!(),
                    }
                })*
                EventKind::UserEvent(_) => {
                    let package = UserEventPackage::<R>::flattened(event).unwrap_or_else(|err| {
                        panic!("{:?}", err)
                    });
                    FlatEvent::UserEventPackage(package)
                }
            }
        }
    }
}

/// Generates the FlatEvent enum starting from a list of event identifiers.
macro_rules! flat_event {
    ($( $x:ident, $ser:expr, $de:expr ),* $(,)?) => {
        /// An enum representation of event trait objects.
        #[derive(Serialize, Deserialize)]
        pub enum FlatEvent<R: BattleRules> {
            $(#[allow(missing_docs)]
            #[serde(bound(
                serialize = $ser,
                deserialize = $de
            ))]
            $x($x<R>),)*
            #[allow(missing_docs)]
            #[serde(bound(
                serialize = "UserEventPackage<R>: Serialize",
                deserialize = "UserEventPackage<R>: Deserialize<'de>"
            ))]
            UserEventPackage(UserEventPackage<R>),
        }

        impl<R: BattleRules + 'static> FlatEvent<R> {
            flat_event_boxed! { $($x),* }

            flat_event_flattened! { $($x),* }
        }
    };
}

flat_event! {
    DummyEvent, "DummyEvent<R>: Serialize", "DummyEvent<R>: Deserialize<'de>",
    CreateTeam, "CreateTeam<R>: Serialize", "CreateTeam<R>: Deserialize<'de>",
    CreateCreature, "CreateCreature<R>: Serialize", "CreateCreature<R>: Deserialize<'de>",
    CreateObject, "CreateObject<R>: Serialize", "CreateObject<R>: Deserialize<'de>",
    MoveEntity, "MoveEntity<R>: Serialize", "MoveEntity<R>: Deserialize<'de>",
    StartTurn, "StartTurn<R>: Serialize", "StartTurn<R>: Deserialize<'de>",
    EndTurn, "EndTurn<R>: Serialize", "EndTurn<R>: Deserialize<'de>",
    EndRound, "EndRound<R>: Serialize", "EndRound<R>: Deserialize<'de>",
    EnvironmentTurn, "EnvironmentTurn<R>: Serialize", "EnvironmentTurn<R>: Deserialize<'de>",
    ActivateAbility, "ActivateAbility<R>: Serialize", "ActivateAbility<R>: Deserialize<'de>",
    ApplyImpact, "ApplyImpact<R>: Serialize", "ApplyImpact<R>: Deserialize<'de>",
    AlterStatistics, "AlterStatistics<R>: Serialize", "AlterStatistics<R>: Deserialize<'de>",
    AlterStatuses, "AlterStatuses<R>: Serialize", "AlterStatuses<R>: Deserialize<'de>",
    AlterAbilities, "AlterAbilities<R>: Serialize", "AlterAbilities<R>: Deserialize<'de>",
    AlterPowers, "AlterPowers<R>: Serialize", "AlterPowers<R>: Deserialize<'de>",
    RegenerateStatistics, "RegenerateStatistics<R>: Serialize", "RegenerateStatistics<R>: Deserialize<'de>",
    RegenerateAbilities, "RegenerateAbilities<R>: Serialize", "RegenerateAbilities<R>: Deserialize<'de>",
    RegeneratePowers, "RegeneratePowers<R>: Serialize", "RegeneratePowers<R>: Deserialize<'de>",
    InflictStatus, "InflictStatus<R>: Serialize", "InflictStatus<R>: Deserialize<'de>",
    ClearStatus, "ClearStatus<R>: Serialize", "ClearStatus<R>: Deserialize<'de>",
    ConvertCreature, "ConvertCreature<R>: Serialize", "ConvertCreature<R>: Deserialize<'de>",
    SetRelations, "SetRelations<R>: Serialize", "SetRelations<R>: Deserialize<'de>",
    ConcludeObjectives, "ConcludeObjectives<R>: Serialize", "ConcludeObjectives<R>: Deserialize<'de>",
    RemoveCreature, "RemoveCreature<R>: Serialize", "RemoveCreature<R>: Deserialize<'de>",
    RemoveObject, "RemoveObject<R>: Serialize", "RemoveObject<R>: Deserialize<'de>",
    RemoveTeam, "RemoveTeam<R>: Serialize", "RemoveTeam<R>: Deserialize<'de>",
    AlterSpace, "AlterSpace<R>: Serialize", "AlterSpace<R>: Deserialize<'de>",
    ResetEntropy, "ResetEntropy<R>: Serialize", "ResetEntropy<R>: Deserialize<'de>",
    ResetObjectives, "ResetObjectives<R>: Serialize", "ResetObjectives<R>: Deserialize<'de>",
    ResetRounds, "ResetRounds<R>: Serialize", "ResetRounds<R>: Deserialize<'de>",
    ResetSpace, "ResetSpace<R>: Serialize", "ResetSpace<R>: Deserialize<'de>",
    EndBattle, "EndBattle<R>: Serialize", "EndBattle<R>: Deserialize<'de>",
}

/// A versioned event wrapper containing a flattened event.
/// Use this struct to serialize/deserialize a `VersionedEventWrapper`.
#[derive(Serialize, Deserialize)]
pub struct FlatVersionedEvent<R: BattleRules> {
    id: EventId,
    origin: Option<EventId>,

    #[serde(bound(
        serialize = "FlatEvent<R>: Serialize",
        deserialize = "FlatEvent<R>: Deserialize<'de>"
    ))]
    event: FlatEvent<R>,

    #[serde(bound(
        serialize = "Version<R>: Serialize",
        deserialize = "Version<R>: Deserialize<'de>"
    ))]
    version: Version<R>,
}

impl<R: BattleRules> FlatVersionedEvent<R> {
    /// Returns the id of this event.
    pub fn id(&self) -> EventId {
        self.id
    }

    /// Returns the origin of this event.
    pub fn origin(&self) -> Option<EventId> {
        self.origin
    }

    /// Returns the inner `FlatEvent`.
    pub fn event(&self) -> &FlatEvent<R> {
        &self.event
    }

    /// Returns the rules' version under which this event was created.
    pub fn version(&self) -> &Version<R> {
        &self.version
    }
}

impl<R: BattleRules + 'static> From<VersionedEventWrapper<R>> for FlatVersionedEvent<R> {
    fn from(event: VersionedEventWrapper<R>) -> Self {
        Self {
            id: event.wrapper().id(),
            origin: event.wrapper().origin(),
            event: FlatEvent::flattened(event.wrapper.event),
            version: event.version,
        }
    }
}

impl<R: BattleRules + 'static> From<FlatVersionedEvent<R>> for VersionedEventWrapper<R> {
    fn from(event: FlatVersionedEvent<R>) -> Self {
        Self::new(
            EventWrapper::new(event.id, event.origin, event.event.boxed()),
            event.version,
        )
    }
}

/// A versioned client event containing a flattened event.
/// Use this struct to serialize/deserialize a `ClientEventPrototype`.
#[derive(Serialize, Deserialize)]
pub struct FlatClientEvent<R: BattleRules> {
    origin: Option<EventId>,

    #[serde(bound(
        serialize = "FlatEvent<R>: Serialize",
        deserialize = "FlatEvent<R>: Deserialize<'de>"
    ))]
    event: FlatEvent<R>,

    #[serde(bound(
        serialize = "Version<R>: Serialize",
        deserialize = "Version<R>: Deserialize<'de>"
    ))]
    version: Version<R>,

    player: Option<PlayerId>,
}

impl<R: BattleRules> FlatClientEvent<R> {
    /// Returns the origin of this event.
    pub fn origin(&self) -> Option<EventId> {
        self.origin
    }

    /// Returns the inner `FlatEvent`.
    pub fn event(&self) -> &FlatEvent<R> {
        &self.event
    }

    /// Returns the rules' version under which this event was created.
    pub fn version(&self) -> &Version<R> {
        &self.version
    }

    /// Returns the player to whom this event belongs.
    pub fn player(&self) -> Option<PlayerId> {
        self.player
    }
}

impl<R: BattleRules + 'static> From<ClientEventPrototype<R>> for FlatClientEvent<R> {
    fn from(event: ClientEventPrototype<R>) -> Self {
        let player = event.player();
        Self {
            origin: event.origin(),
            event: FlatEvent::flattened(event.event),
            version: event.version,
            player,
        }
    }
}

impl<R: BattleRules + 'static> From<FlatClientEvent<R>> for ClientEventPrototype<R> {
    fn from(event: FlatClientEvent<R>) -> Self {
        Self::new(
            event.origin,
            event.event.boxed(),
            event.version,
            event.player,
        )
    }
}
