# Performance Tests

Run the server

```shell
cargo r -r -p players
```

In a separate terminal, start [rust-mc-bot](https://github.com/Eoghanmc22/rust-mc-bot).
This command should connect 1000 clients to the server.

```shell
# In the rust-mc-bot directory
cargo r -r -- 127.0.0.1:25565 1000
```

If the delta time is consistently >50ms, the server is running behind schedule.

Note:

# Flamegraph

To start capturing a [flamegraph](https://github.com/flamegraph-rs/flamegraph),
run the server like this:

```shell
# You can also try setting the `CARGO_PROFILE_RELEASE_DEBUG` environment variable to `true`.
cargo flamegraph -p players
```

Run rust-mc-bot as above, and then stop the server after a few seconds. Flamegraph will generate a flamegraph.svg in the
current directory. You can then open that file in your internet browser of choice.
