# Performance Tests

Run the server

```shell
cargo r -r -p players
```

In a separate terminal, start [rust-mc-bot](https://github.com/Eoghanmc22/rust-mc-bot).
This command should connect 1000 clients to the server.

```shell
cargo r -r -- 127.0.0.1:25565 1000

# If rust-mc-bot was cloned in the performance_tests directory, do
cargo r -r -p rust-mc-bot -- 127.0.0.1:25565 1000
```

If the delta time is consistently >50ms, the server is running behind schedule.

# Flamegraph

To start capturing a [flamegraph](https://github.com/flamegraph-rs/flamegraph),
run the server like this:

```shell
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -p players
```

Run rust-mc-bot as above, and then stop the server after a few seconds. Flamegraph will take its own sweet time to
generate a flamegraph.svg in the current directory. You can then open that file in your internet browser of choice.

NOTE: The indiscriminate use of `rayon` in Valence appears to have made the flamegraph basically unreadable. This
situation should change soon.