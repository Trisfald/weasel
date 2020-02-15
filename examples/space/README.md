# Space

In this example we'll discover how to manage the *space dimension* in weasel.

Our space model will start as a two dimensional plane, divided in squares. We will then spawn a few creatures, each one on a different square.

As the next step, deadly traps will be placed across the two diagonals.

Finally, we are going to regenerate the space, transforming the 2D plane into a single line of squares; in other words we drop one dimension.

Run the example with:
```
cargo run --example space
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): rules definition (space rules, in particular).
- [main.rs](main.rs): manages the battle and creates a few events.
