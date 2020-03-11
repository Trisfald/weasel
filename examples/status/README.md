# Status

In this example we will demonstrate how to implement different types of long lasting status effects.

We will first create a creature and an object. Then, we inflict a status effect on the creature that will increase its health as long as it is active. The object, instead, will be dealt damage over time which will reduce its life at each round. Finally, we will see how to end the effects manually or after a certain number of rounds.

Run the example with:
```
cargo run --example status
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): rules definition.
- [main.rs](main.rs): manages the displayed messages and handles the battle server.
