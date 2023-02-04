<img src="assets/logo-full.svg" width="650">

_**NOTE:** Valence is currently undergoing a major rewrite. The information in this README may be outdated. See [ECS Rewrite](https://github.com/valence-rs/valence/pull/184) for more information._

---

A Rust framework for building Minecraft: Java Edition servers.

Like [feather](https://github.com/feather-rs/feather), Valence is an effort to build a Minecraft compatible server
completely from scratch in Rust. The difference is that Valence has decided to organize the effort a little differently.
All game logic is behind a trait. This approach has many advantages. Features such as a plugin system, dedicated
executable, and vanilla game mechanics can be implemented _on top of_ Valence. Valence is a Rust library like any other.

In the future we may decide to reimplement vanilla game mechanics as a separate project. If you're developing something
like a minigame server without need for vanilla game mechanics, you can depend on Valence directly.

# Goals

Valence aims to be the following:

* **Complete**. Abstractions for the full breadth of the Minecraft protocol.
* **Flexible**. Valence provides direct access to Minecraft's protocol when necessary.
* **Minimal**. The API surface is small with only the necessities exposed. Opinionated features such as a
  standalone executable, plugin system, and reimplementation of vanilla mechanics should be built in a separate project on
  top of the foundation that Valence provides.
* **Intuitive**. An API that is easy to use and difficult to misuse. Extensive documentation is important.
* **Efficient**. Optimal use of system resources with multiple CPU cores in mind.
* **Up to date**. Targets the most recent stable version of Minecraft. Support for multiple versions at once is not
  planned (although you can use a proxy).

## Current Status

Valence is still early in development with many features unimplemented or incomplete. However, the foundations are in
place. Here are some noteworthy achievements:

- [x] A new serde library for Minecraft's Named Binary Tag (NBT) format
- [x] Authentication, encryption, and compression
- [x] Block states
- [x] Chunks
- [x] Entities and tracked data
- [x] Bounding volume hierarchy for fast spatial entity queries
- [x] Player list and player skins
- [x] Dimensions, biomes, and worlds
- [x] JSON Text API
- [x] A Fabric mod for extracting data from the game into JSON files. These files are processed by a build script to
  generate Rust code for the project. The JSON files can be used in other projects as well.
- [x] Items
- [x] Particles
- [x] Anvil file format (read only)
- [ ] Inventory
- [ ] Block entities
- [x] Proxy support ([Velocity](https://velocitypowered.com/), [Bungeecord](https://www.spigotmc.org/wiki/bungeecord/) and [Waterfall](https://docs.papermc.io/waterfall))
- [ ] Utilities for continuous collision detection

Here is a [short video](https://www.youtube.com/watch?v=6P072lKE01s) showing the examples and some of its current
capabilities.

# Getting Started

## Running the Examples

You may want to try running one of the examples. After cloning the repository, run

```shell
cargo r -r --example conway
```

Next, open your Minecraft client and connect to the address `localhost`.
If all goes well you should be playing on the server.

## Adding Valence as a Dependency

Valence is published to [crates.io](https://crates.io/crates/valence). Run `cargo add valence` to add it to your
project. Documentation is available [here](https://docs.rs/valence/latest/valence/).

However, the crates.io version is likely outdated. To use the most recent development version, add Valence as a
[git dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories)
.

```toml
[dependencies]
valence = { git = "https://github.com/valence-rs/valence" }
```

View the documentation by running `cargo d --open` in your project.

# Contributing

Contributions are welcome! Please
see [CONTRIBUTING.md](https://github.com/valence-rs/valence/blob/main/CONTRIBUTING.md). You can also
join [the Discord](https://discord.gg/8Fqqy9XrYb) to discuss the project and ask questions.

# License

Code is licensed under [MIT](https://opensource.org/licenses/MIT) while the Valence logo is
under [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/)

# Funding

If you would like to contribute financially consider sponsoring me (rj00a)
on [GitHub](https://github.com/sponsors/rj00a)
or [Patreon](https://www.patreon.com/rj00a) (GitHub is preferred).

I would love to continue working on Valence and your support would help me do that. Thanks!
