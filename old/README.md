<p align="center">
    <img src="https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo-full.svg" width="650" align="center">
</p>

<p align="center">
    <a href="https://github.com/valence-rs/valence/blob/main/LICENSE.txt">
        <img src="https://img.shields.io/github/license/valence-rs/valence"
            alt="license"></a>
    <a href="https://crates.io/crates/valence">
      <img src="https://img.shields.io/crates/d/valence?label=crates.io"></a>
    <a href="https://discord.gg/8Fqqy9XrYb">
        <img src="https://img.shields.io/discord/998132822239870997?logo=discord"
            alt="chat on Discord"></a>
    <a href="https://github.com/sponsors/rj00a">
        <img src="https://img.shields.io/github/sponsors/rj00a"
            alt="GitHub sponsors"></a>
</p>

A Rust framework for building Minecraft: Java Edition servers.

Built on top of [Bevy ECS](https://bevyengine.org/learn/book/getting-started/ecs/), Valence is an effort to create a
Minecraft compatible server completely from scratch in Rust. You can think of Valence as a _game engine for
Minecraft servers_. It doesn't do much by default, but by writing game logic yourself and leveraging Bevy's
powerful [plugin system](https://bevyengine.org/learn/book/getting-started/plugins/), you can make almost anything.

Opinionated features like dynamic scripting, dedicated executables, and vanilla game mechanics are all expected to be
built as optional plugins. This level of modularity is desirable for those looking to build highly custom experiences
in Minecraft such as minigame servers.

⚠️ **Valence is still early in development with many features unimplemented or incomplete. Expect to encounter bugs, limitations, and breaking changes.**

# Goals

Valence aims to be the following:

* **Complete**. Abstractions for the full breadth of the Minecraft protocol.
* **Flexible**. Can easily extend Valence from within user code. Direct access to the Minecraft protocol is provided.
* **Modular**. Pick and choose the components you need.
* **Intuitive**. An API that is easy to use and difficult to misuse. Extensive documentation and examples are important.
* **Efficient**. Optimal use of system resources with multiple CPU cores in mind. Valence uses very little memory and
  can
  support [thousands](https://raw.githubusercontent.com/valence-rs/valence/main/assets/many-players.png)
  of players at the same time without lag (assuming you have the bandwidth).
* **Up to date**. Targets the most recent stable version of Minecraft. Support for multiple versions at once is not
  planned. However, you can use a proxy with [ViaBackwards](https://www.spigotmc.org/resources/viabackwards.27448/) to
  achieve backwards compatibility with older clients.

## Current Status

Here are some noteworthy achievements:
- `valence_nbt`: A speedy new library for Minecraft's Named Binary Tag (NBT) format.
- Authentication, encryption, and compression
- Block states
- Chunks
- Entities and metadata
- Bounding volume hierarchy for fast spatial entity queries
- Player list and player skins
- Dimensions, biomes, and worlds
- JSON Text API
- A Fabric mod for extracting data from the game into JSON files. These files are processed by a build script to
  generate Rust code for the project. The JSON files can be used in other projects as well.
- Inventories
- Items
- Particles
- Anvil file format (read only)
- Proxy support ([Velocity](https://velocitypowered.com/), [Bungeecord](https://www.spigotmc.org/wiki/bungeecord/)
  and [Waterfall](https://docs.papermc.io/waterfall))

Here is a [short video](https://www.youtube.com/watch?v=jkw9fZx9Etg) showing the examples and some of
Valence's capabilities.

# Getting Started

## Running the Examples

After cloning the repository, run this command to try an example.

```shell
cargo r -r --example parkour
```

 I also recommend giving `game_of_life`, `terrain`, and `cow_sphere` a try.

Next, open your Minecraft client and connect to the address `localhost`.
If all goes well you should be playing on the server.

## Adding Valence as a Dependency

Valence is published to [crates.io](https://crates.io/crates/valence). Run `cargo add valence` to add it to your
project.

However, the crates.io version is likely outdated. To use the most recent development version, add Valence as a
[git dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories).

```toml
[dependencies]
valence = { git = "https://github.com/valence-rs/valence" }
```

Documentation from the main branch is available [here](https://valence.rs/rustdoc/valence/).

# Contributing

Contributions are welcome! Please
see [CONTRIBUTING.md](https://github.com/valence-rs/valence/blob/main/CONTRIBUTING.md). You can
join [Discord](https://discord.gg/8Fqqy9XrYb) or [GitHub Discussions](https://github.com/valence-rs/valence/discussions)
to discuss the project and ask questions.

# License

Code is licensed under [MIT](https://opensource.org/licenses/MIT) while the Valence logo is
under [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/)

# Funding

If you would like to contribute financially, consider sponsoring me (rj00a)
on [GitHub](https://github.com/sponsors/rj00a)
or [Patreon](https://www.patreon.com/rj00a).

I would love to continue working on Valence and your support would help me do that. Thanks!
