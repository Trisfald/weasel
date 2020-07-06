# King of the hill

A multiplayer card game designed for three players.

One player must act as server:
```
cargo run --example king --all-features
```

The other players must connect to the server:
```
cargo run --example king --all-features <ip address of server>:3000
```

## The rules

The rules are extremely simple to keep the focus on how to use the library, not on the game itself. Any similarity with existing games is purely coincidental.
- The game starts with three players and a deck of nine cards numbered from 1 to 9.
- Each player is given three cards, randomly.
- During each turn, for a total of three turns, every player chooses a card to play.
- The highest card wins.
- The player with the most number of wins is the game's winner.

Clients can leave the game and reconnect to resume playing!

## Let's get to business

The *King of hill* game is implemented in three source code files:
- [rules.rs](rules.rs): contains all rules for our card game.
- [main.rs](main.rs): all the necessary code to handle player input, textual output and game progress.
- [tpc.rs](tpc.rs): manages networking between players.
