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
use crate::player::PlayerId;
use crate::round::{EndRound, ResetRounds, StartRound};
use crate::space::{AlterSpace, MoveEntity, ResetSpace};
use crate::team::{ConcludeObjectives, CreateTeam, RemoveTeam, ResetObjectives, SetRelations};
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
        pub fn boxed(self) -> Box<dyn Event<R>> {
            // Generate a match with an arm for every concrete event type.
            match self {
                $(FlatEvent::$x(flat) => {
                    Box::new(flat) as Box<dyn Event<R>>
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
        pub fn flattened(event: Box<dyn Event<R>>) -> FlatEvent<R> {
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
    StartRound, "StartRound<R>: Serialize", "StartRound<R>: Deserialize<'de>",
    EndRound, "EndRound<R>: Serialize", "EndRound<R>: Deserialize<'de>",
    CreateTeam, "CreateTeam<R>: Serialize", "CreateTeam<R>: Deserialize<'de>",
    CreateCreature, "CreateCreature<R>: Serialize", "CreateCreature<R>: Deserialize<'de>",
    ActivateAbility, "ActivateAbility<R>: Serialize", "ActivateAbility<R>: Deserialize<'de>",
    ResetEntropy, "ResetEntropy<R>: Serialize", "ResetEntropy<R>: Deserialize<'de>",
    MoveEntity, "MoveEntity<R>: Serialize", "MoveEntity<R>: Deserialize<'de>",
    ApplyImpact, "ApplyImpact<R>: Serialize", "ApplyImpact<R>: Deserialize<'de>",
    AlterStatistics, "AlterStatistics<R>: Serialize", "AlterStatistics<R>: Deserialize<'de>",
    AlterAbilities, "AlterAbilities<R>: Serialize", "AlterAbilities<R>: Deserialize<'de>",
    SetRelations, "SetRelations<R>: Serialize", "SetRelations<R>: Deserialize<'de>",
    ConvertCreature, "ConvertCreature<R>: Serialize", "ConvertCreature<R>: Deserialize<'de>",
    EndBattle, "EndBattle<R>: Serialize", "EndBattle<R>: Deserialize<'de>",
    ConcludeObjectives, "ConcludeObjectives<R>: Serialize", "ConcludeObjectives<R>: Deserialize<'de>",
    ResetObjectives, "ResetObjectives<R>: Serialize", "ResetObjectives<R>: Deserialize<'de>",
    ResetRounds, "ResetRounds<R>: Serialize", "ResetRounds<R>: Deserialize<'de>",
    ResetSpace, "ResetSpace<R>: Serialize", "ResetSpace<R>: Deserialize<'de>",
    RemoveCreature, "RemoveCreature<R>: Serialize", "RemoveCreature<R>: Deserialize<'de>",
    RemoveTeam, "RemoveTeam<R>: Serialize", "RemoveTeam<R>: Deserialize<'de>",
    RegenerateStatistics, "RegenerateStatistics<R>: Serialize", "RegenerateStatistics<R>: Deserialize<'de>",
    RegenerateAbilities, "RegenerateAbilities<R>: Serialize", "RegenerateAbilities<R>: Deserialize<'de>",
    AlterSpace, "AlterSpace<R>: Serialize", "AlterSpace<R>: Deserialize<'de>",
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

impl<R: BattleRules + 'static> From<VersionedEventWrapper<R>> for FlatVersionedEvent<R> {
    fn from(event: VersionedEventWrapper<R>) -> Self {
        FlatVersionedEvent {
            id: event.wrapper().id,
            origin: event.wrapper().origin,
            event: FlatEvent::flattened(event.wrapper.event),
            version: event.version,
        }
    }
}

impl<R: BattleRules + 'static> From<FlatVersionedEvent<R>> for VersionedEventWrapper<R> {
    fn from(event: FlatVersionedEvent<R>) -> Self {
        VersionedEventWrapper::new(
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

impl<R: BattleRules + 'static> From<ClientEventPrototype<R>> for FlatClientEvent<R> {
    fn from(event: ClientEventPrototype<R>) -> Self {
        let player = event.player();
        FlatClientEvent {
            origin: event.origin(),
            event: FlatEvent::flattened(event.event),
            version: event.version,
            player,
        }
    }
}

impl<R: BattleRules + 'static> From<FlatClientEvent<R>> for ClientEventPrototype<R> {
    fn from(event: FlatClientEvent<R>) -> Self {
        ClientEventPrototype::new(
            event.origin,
            event.event.boxed(),
            event.version,
            event.player,
        )
    }
}
