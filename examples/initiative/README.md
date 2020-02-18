# Initiative

This example shows how to implement `RoundsRules` to decide the order of acting during a battle.

The first step is to create five creatures, each one with a different value of *speed*. Then, we will repeatedly start and end rounds while also displaying the global order of initiative.

Run the example with:
```
cargo run --example initiative --all-features
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): rules definition (round rules, in particular).
- [main.rs](main.rs): manages the battle and creates a few events.
