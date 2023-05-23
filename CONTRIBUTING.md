Valence has a public Discord server [here](https://discord.gg/8Fqqy9XrYb). Check it out if you have additional questions
or comments.

# What version of Rust should I use?

To _use_ Valence, only the most recent stable version of Rust is required. However, contributors should know that
unstable `rustfmt` settings are enabled in the project. To run `rustfmt` with the nightly toolchain, use
the `cargo +nightly fmt` command.

# What issues can I work on?

Issues
labelled [good first issue](https://github.com/valence-rs/valence/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
are a good place to start. This label is reserved for issues that shouldn't require too much specialized domain
knowledge to complete. New contributors are not required to start with these issues.

If you plan to work on something that's not an open issue, consider making one first so that it can be discussed. This
way, your contribution will not be rejected when it is submitted for review.

## Playgrounds

Playgrounds are meant to provide a quick and minimal environment to test out new code or reproduce bugs. Playgrounds are also a great way test out quick ideas. This is the preferred method for providing code samples in issues and pull requests.

To get started with a new playground, copy the template to `playground.rs`.

```bash
cp tools/playground/src/playground.template.rs tools/playground/src/playground.rs
```

Make your changes to `crates/playground/src/playground.rs`. To run it:

```bash
cargo run -p playground # simply run the playground, or
cargo watch -c -x "run -p playground" # run the playground and watch for changes
```

# Automatic Checks

When you submit a pull request, your code will automatically run through clippy, rustfmt, etc. to check for any errors.
If an error does occur, it must be fixed before the pull request can be merged.

# Code Conventions

Here are some rules you should follow for your code. Generally the goal here is to be consistent with existing code, the
standard library, and the Rust ecosystem as a whole. Nonconforming code is not necessarily a blocker for accepting your
contribution, but conformance is advised.

These guidelines are intended to complement
the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html).

## Top-down Modules

Readers of the module should be able to understand your code by reading it from top to bottom.
Whenever [items](https://doc.rust-lang.org/reference/items.html) in your module form a parent-child relationship, the
parent should be written above the children. Typically this means that important `pub` items are placed before private
implementation details.

For instance, here are three functions. Notice how the definition of `foo` is placed above its dependencies. The parent
is `foo` while its children are `bar` and `baz`.

```rust
pub fn foo() {
    bar();
    baz();
}

fn bar() {}

fn baz() {}
```

This guideline applies to types as well.

```rust
pub struct Foo {
    bars: Vec<Bar>,
}

struct Bar {
    // ...
}
```

## Getters and Setters

Getters and setters should be named like this:

```rust
impl Foo {
    fn bar(&self) -> &Bar { ... }
    fn set_bar(&mut self, bar: Bar) { ... }
}
```

And **not** like this:

```rust
impl Foo {
    fn get_bar(&self) -> &Bar { ... }
    fn set_bar(&mut self, bar: Bar) { ... }
}
```

See [`SocketAddr`](https://doc.rust-lang.org/stable/std/net/enum.SocketAddr.html) for an example of a standard library
type that uses this convention.

Under appropriate circumstances a different naming scheme can be
used. [`Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html) is a standard type that demonstrates
this.

If a `bar` field exists and no invariants need to be maintained by the getters and setters, it is usually better to make
the `bar` field public.

## Bevy `Event` naming conventions

Types intended to be used as events in [`EventReader`] and [`EventWriter`] should end in the `Event` suffix.
This is helpful for readers trying to distinguish events from other types in the program.

Good:
```rust
struct CollisionEvent { ... }

fn handle_collisions(mut events: EventReader<CollisionEvent>) { ... }
```

Bad:
```rust
struct Collision { ... }

fn handle_collisions(mut events: EventReader<Collision>) { ... }
```

[`EventReader`]: https://docs.rs/bevy_ecs/latest/bevy_ecs/event/struct.EventReader.html
[`EventWriter`]: https://docs.rs/bevy_ecs/latest/bevy_ecs/event/struct.EventWriter.html

## Specifying Dependencies

When adding a new dependency to a crate, make sure you specify the full semver version.

Do this:
```toml
[dependencies]
serde_json = "1.0.96"
```

And _not_ this:
```toml
[dependencies]
serde_json = "1"
```

## Documentation

All public items should be documented. Documentation must be written with complete sentences and correct grammar.
Consider using [intra-doc links](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
where appropriate.

## Unit Tests

Unit tests help your contributions last! They ensure that your code works as expected and that it continues to work in
the future.

whole-server unit tests can be found in [`crates/valence/src/tests/`](crates/valence/src/tests).

## Naming Quantities

Quantities of something should be named `foo_count` where `foo` is the thing you're quantifying. It would be incorrect
to name this variable `num_foos`.
