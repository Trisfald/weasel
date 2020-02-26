# Passive

In this example we implement a simple passive ability. This skill increases the power of another ability at every round.

At the start we'll spawn two soldiers. Both of them known the ability *punch*, but only one has the passive ability *power up*.

Every time the soldier ends his round, *power up* increases the power of *punch* by one times the number of creatures on the battlefield.

Run the example with:
```
cargo run --example passive
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): actor rules definition.
- [main.rs](main.rs): manages the battle and creates a few events.
