# Pirates

A simple singleplayer game to demonstrate the basic capabilities of `weasel`. The game has very simple rules, thus it's a good example for people to understand how `weasel` works.

Run the example with:
```
cargo run --example pirates --all-features
```

## The objective

Create a simple game with the following characteristics:
- Two teams, one controlled by the computer.
- Each team has one ship.
- Ships' position doesn't matter.
- Ships have values for hull (100) and crew (100).
- Ships have two abilities: one to damage the hull and another to damage the crew.
- Damage of attacks is randomized between 10 + crew/20 and 10 + crew/5.
- Ships sink when their hull reaches 0.
- The last team standing will be the winner.
- At the start of each player turn, it's possible to save the game.
- Savestates can be loaded at any time.

## Let's get to business

The *Pirates* game is implemented in three source code files:
- [rules.rs](rules.rs): contains all rules for the battle system.
- [game.rs](game.rs): manages a running game.
- [main.rs](main.rs): all the necessary code to handle player input, textual output and initialization.

During the game you will have to possibility to create or load a savestate. The latter is persisted to disk in `/tmp/savegame`.\
Since the file is saved in json format, you can open it and have a look at the timeline of events.
