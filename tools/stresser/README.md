# stresser

A Minecraft client for testing server performance under heavy load. (Incomplete)

## Usage

```
# Run the example in valence
cargo run --example bench_players

# Run the stressor tool in tools/stresser
cargo run -p stresser -- --target 127.0.0.1:25565 --count 1000
```