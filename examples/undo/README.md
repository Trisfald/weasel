# Undo

In this example the player can move a creature on a two dimensional space. He will be able to undo or redo his moves.

As you will see in the code, we use a cheap trick to implement the undo mechanics. That is, we replay the history up to the last completed round.\
Doing so should be fine if your battles are quite short. In the case of complex and long fights replaying the history might take a not negligible amount of time. 

Run the example with:
```
cargo run --example undo
```

The program is implemented in two source code files:
- [rules.rs](rules.rs): rules definition.
- [main.rs](main.rs): manages the battle, the player input and implements the undo/redo mechanism.
