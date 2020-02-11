# Autosave

An example showing how to use an event sink to populate an autosave.\
The same pattern can be used to send events to another destination.

Remember that there are other ways to create savestates, which in certain situations may be better than the one described in this example. For instance, you can manually create a new savestate after each player action or at any other arbitrary moment.\
If you really care about ensuring that player's progress is not lost, it's better to keep several files and rotate them.

Run the example with:
```
cargo run --example autosave --all-features
```

The program is implemented in two source code files:
- [sink.rs](sink.rs): a sink to dump events to a file.
- [main.rs](main.rs): user input, output messages and managing of the battle.

The autosave is persisted to disk in `/tmp/autosave`.\
Since the file is saved in json format, you can open it and have a look at the timeline of events.
