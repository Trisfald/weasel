#![deny(missing_docs)]

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
//! use weasel::{Server, battle_rules, rules::empty::*};
//! use weasel::battle::{Battle, BattleRules};
//! use weasel::team::CreateTeam;
//! use weasel::event::EventTrigger;
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

pub use crate::client::Client;
pub use crate::error::{WeaselError, WeaselResult};
pub use crate::server::Server;

pub mod ability;
pub mod actor;
pub mod battle;
pub mod character;
pub mod client;
pub mod creature;
pub mod entity;
pub mod entropy;
pub mod error;
pub mod event;
pub mod fight;
pub mod history;
pub mod metric;
pub mod player;
pub mod round;
pub mod rules;
#[cfg(feature = "serialization")]
pub mod serde;
pub mod server;
pub mod space;
pub mod team;
pub mod user;
pub mod util;
