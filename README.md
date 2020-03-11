# Weasel Turn Battle System
[![Build Status](https://travis-ci.org/Trisfald/weasel.svg?branch=master)](https://travis-ci.org/Trisfald/weasel)
[![Code Coverage](https://codecov.io/gh/Trisfald/weasel/branch/master/graph/badge.svg)](https://codecov.io/gh/Trisfald/weasel)
[![crates.io](https://meritbadge.herokuapp.com/weasel)](https://crates.io/crates/weasel)
[![Released API docs](https://docs.rs/weasel/badge.svg)](https://docs.rs/weasel)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

weasel is a customizable battle system for turn-based games.

* Simple way to define the combat's rules, taking advantage of Rust's strong type system.
* Battle events are collected into a timeline to support save and restore, replays, and more.
* Client/server architecture; all battle events are verified by the server.
* Minimal performance overhead.

## Examples

```rust
use weasel::{Server, battle_rules, rules::empty::*};
use weasel::battle::{Battle, BattleRules};
use weasel::team::CreateTeam;
use weasel::event::EventTrigger;

battle_rules! {}

let battle = Battle::builder(CustomRules::new()).build(); 
let mut server = Server::builder(battle).build();

CreateTeam::trigger(&mut server, 1).fire().unwrap();
assert_eq!(server.battle().entities().teams().count(), 1);
```

You can find real examples of battle systems made with weasel in [examples](examples/).

## How does it work?

To use this library, you would create instances of its main objects: `server` and `client`.
You will notice that both of them are parameterized with a `BattleRules` generic type.\
A `server` is mandatory to manage a game. A server can be also a client.
For example, a typical single player game needs only one server.\
A `client` is a participant to a game. It sends commands to a server on behalf of a player.
A multiplayer game would have one server and multiple clients.

Once you have instantiated a `server` and possibly one or more `clients`,
you are ready to begin a new game.\
Games are carried forward by creating `events`.
There are many kind of events, see the documentation to know more.

Through a `server` or a `client` you'll be able to access the full state of the battle,
including the entire timeline of events.

## Features

weasel provides many functionalities to ease the development of a turn based game:

- Creatures and inanimate objects.
- Statistics and abilities for characters.
- Long lasting status effects.
- Player managed teams.
- Team objectives and diplomacy.
- Division of the battle into rounds.
- Rules to govern the game subdivided into orthogonal traits.
- Fully serializable battle history.
- Cause-effect relationship between events.
- Server side verification of clients' events.
- Player permissions and authorization.
- Versioning for battle rules.
- User defined events.
- System and user defined metrics.
- Sinks to forward events to an arbitrary destination.
- Small collection of predefined rules.

## Contributing

Thanks for your interest in contributing! There are many ways to contribute to this project. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

weasel is provided under the MIT license. See [LICENSE](LICENSE).
