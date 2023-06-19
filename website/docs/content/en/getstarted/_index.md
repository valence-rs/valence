+++
title = "Getting Started"
weight = 2
type = "docs"
description = '''
Welcome to the Valence docs
'''
+++
## Running Examples
After cloning the repository, view the list of examples by running:
```
cargo r -r --example
```

Next, open your Minecraft client and connect to the address localhost. If all goes well you should be playing on the server.

## Adding Valence as a Dependency

Valence is published to [crates.io](https://crates.io/crates/valence). Run `cargo add valence` to add it to your
project. Documentation is available [here](https://docs.rs/valence/latest/valence/).

However, the crates.io version is likely outdated. To use the most recent development version, add Valence as a
[git dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories)

```toml
[dependencies]
valence = { git = "https://github.com/valence-rs/valence" }
```

View the latest documentation by running `cargo d --open` in your project.