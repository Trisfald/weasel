#![deny(missing_docs)]
#![doc(test(attr(warn(warnings))))]

//!
//! weasel is a customizable battle system for turn-based games.
//!
//! * Simple way to define the combat's rules, taking advantage of Rust's strong type system.
//! * Battle events are collected into a timeline to support save and restore, replays, and more.
//! * Client/server architecture; all battle events are verified by the server.
//! * Minimal performance overhead.
//!
//! ## Examples
//!
//! ```
//! use weasel::{
//!     battle_rules, rules::empty::*, Battle, BattleController,
//!     BattleRules, CreateTeam, EventTrigger, Server,
//! };
//!
//! battle_rules! {}
//!
//! let battle = Battle::builder(CustomRules::new()).build();
//! let mut server = Server::builder(battle).build();
//!
//! CreateTeam::trigger(&mut server, 1).fire().unwrap();
//! assert_eq!(server.battle().entities().teams().count(), 1);
//! ```
//!
//! You can find real examples of battle systems made with weasel in
//! [examples](https://github.com/Trisfald/weasel/tree/master/examples/).
//!
//! ## How does it work?
//!
//! To use this library, you would create instances of its main objects: `server` and `client`.
//! You will notice that both of them are parameterized with a `BattleRules` generic type.\
//! A `server` is mandatory to manage a game. A server can be also a client.
//! For example, a typical single player game needs only one server.\
//! A `client` is a participant to a game. It sends commands to a server on behalf of a player.
//! A multiplayer game would have one server and multiple clients.
//!
//! Once you have instantiated a `server` and possibly one or more `clients`,
//! you are ready to begin a new game.\
//! Games are carried forward by creating `events`.
//! There are many kind of events, see the documentation to know more.
//!
//! Through a `server` or a `client` you'll be able to access the full state of the battle,
//! including the entire timeline of events.
//!
//! ## Features
//!
//! weasel provides many functionalities to ease the development of a turn based game:
//!
//! - Creatures and inanimate objects.
//! - Statistics and abilities for characters.
//! - Long lasting status effects.
//! - Player managed teams.
//! - Team objectives and diplomacy.
//! - Division of the battle into turns and rounds.
//! - Rules to govern the game subdivided into orthogonal traits.
//! - Fully serializable battle history.
//! - Cause-effect relationship between events.
//! - Server side verification of clients' events.
//! - Player permissions and authorization.
//! - Versioning for battle rules.
//! - User defined events.
//! - System and user defined metrics.
//! - Sinks to forward events to an arbitrary destination.
//! - Small collection of predefined rules.
//!
//! ## Define the game's rules via traits
//!
//! `BattleRules` is a collection of modules and it lets you define all the *rules* for your game by
//! implementing a trait for each module.\
//! Having multiple modules helps you in decomposing your rules into smaller parts, orthogonal to
//! each other.
//!
//! ### Predefined rules traits
//!
//! weasel contains a minimal set of predefined rules traits, mainly comprised of rules that do
//! nothing and of basic rules for entropy.
//!
//! You can find the predefined rules in the `::rules` scope.
//!
//! ## Event based
//!
//! weasel is fully based on events. It means that all changes on the battle state must be done
//! through events.\
//! Thanks to this strong restriction, the library can collect all events into a historical
//! timeline. This timeline can then be exported and re-imported at a later stage;
//! this's fundamental to implement save and load or even replays.
//!
//! Users can register on a callback each time an event is processed, to extend the library's
//! functionalities with their own logic.
//!
//! It's possible to create your own events, by implementing the `Event` trait and using the
//! reserved `EventKind::UserEvent`. Remember to also write a `UserEventPacker` in the case
//! you wish to enable serialization.
//!
//! ## Client - server architecture
//!
//! The library uses a client - server architecture to support multiplayer games. Both server
//! and clients contain a replica of the battle's state, but only the events verified by the server
//! will be able to change the state. Client late connection and reconnection are supported.
//!
//! It is necessary that all peers use the same version of the rules.
//!
//! ## Metrics
//!
//! There's a built-in storage for metrics that let you retrieve and modify individual metrics
//! based on an unique id. Metrics are divided in two kind: system and user defined.
//!
//! System metrics are predefined and handled by the library. You can only read their current value.
//!
//! User defined metrics can be created on the fly. The user has full power over them: they can be
//! removed, modified and read.
//!
//! # Optional Features
//!
//! The following optional features are available:
//!
//! - `random`: enables built-in entropy rules that use a pseudorandom number generator.
//! - `serialization`: enables serialization and deserialization of events.

pub mod ability;
pub use crate::ability::ActivateAbility;

pub mod actor;
pub use crate::actor::{Action, Actor, ActorRules, AlterAbilities, RegenerateAbilities};

pub mod battle;
pub use crate::battle::{
    Battle, BattleController, BattleRules, BattleState, EndBattle, EventCallback, Version,
};

pub mod character;
pub use crate::character::{AlterStatistics, Character, CharacterRules, RegenerateStatistics};

pub mod client;
pub use crate::client::Client;

pub mod creature;
pub use crate::creature::{ConvertCreature, CreateCreature, Creature, RemoveCreature};

pub mod entity;
pub use crate::entity::{Entities, Entity, EntityId, RemoveEntity, Transmutation};

pub mod entropy;
pub use crate::entropy::{Entropy, EntropyRules, ResetEntropy};

pub mod error;
pub use crate::error::{WeaselError, WeaselResult};

pub mod event;
pub use crate::event::{
    ClientEventPrototype, Event, EventId, EventKind, EventProcessor, EventPrototype, EventQueue,
    EventReceiver, EventRights, EventServer, EventTrigger, EventWrapper, LinkedQueue,
    VersionedEventWrapper,
};

pub mod fight;
pub use crate::fight::{ApplyImpact, FightRules};

pub mod history;
pub use crate::history::History;

pub mod metric;
pub use crate::metric::{Metric, MetricId, ReadMetrics, SystemMetricId, WriteMetrics};

pub mod object;
pub use crate::object::{CreateObject, Object, RemoveObject};

pub mod player;
pub use crate::player::PlayerId;

pub mod power;
pub use crate::power::InvokePower;

pub mod round;
pub use crate::round::{
    EndRound, EndTurn, EnvironmentTurn, ResetRounds, Rounds, RoundsRules, StartTurn,
};

pub mod rules;

#[cfg(feature = "serialization")]
pub mod serde;
#[cfg(feature = "serialization")]
pub use crate::serde::{FlatClientEvent, FlatEvent, FlatVersionedEvent};

pub mod server;
pub use crate::server::Server;

pub mod space;
pub use crate::space::{AlterSpace, MoveEntity, PositionClaim, ResetSpace, Space, SpaceRules};

pub mod status;
pub use crate::status::{AlterStatuses, Application, AppliedStatus, ClearStatus, InflictStatus};

pub mod team;
pub use crate::team::{
    AlterPowers, Call, ConcludeObjectives, Conclusion, CreateTeam, EntityAddition,
    RegeneratePowers, Relation, RemoveTeam, ResetObjectives, SetRelations, Team, TeamRules,
};

pub mod user;
#[cfg(feature = "serialization")]
pub use crate::user::UserEventPacker;
pub use crate::user::{UserEventId, UserRules};

pub mod util;
pub use crate::util::Id;
