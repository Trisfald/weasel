# User event

This example is a small program that shows how use user defined events and metrics.

First, we define our own `UserRules`, a custom event `MakePizza`. Then we create a `server` and fire two `MakePizza` events.\
Before exiting, the program prints to the terminal the json serialized content of the battle history.

Run the example with:
```
cargo run --example user-event --all-features
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): rules definition (user rules, in particular).
- [main.rs](main.rs): manages the battle and creates a few events.
