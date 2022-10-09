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

# Automatic Checks

When you submit a pull request, your code will automatically run through clippy, rustfmt, etc. to check for any errors.
If an error does occur, it must be fixed before the pull request can be merged.

# Code Conventions

Here are some rules you should follow for your code. Generally the goal here is to be consistent with existing code, the
standard library, and the Rust ecosystem as a whole.

These guidelines are intended to complement
the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html).

## Top-down Modules

[Items](https://doc.rust-lang.org/reference/items.html) in modules should be structured in a top-down style. Readers of
the module should be able to understand your code by reading it from top to bottom. This implies that `pub` items are
placed at the top of the file.

For instance, here are three functions. Notice how the definition of `foo` is placed above its dependencies.

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
    bar: Bar,
    baz: Baz,
}

struct Bar;

struct Baz;
```

## Separate Data and Functions

Types that are closely related should be grouped together separately from the functions that operate on them. `impl`
blocks are placed below the type definitions.

Here is an example combined with the previous guideline:

```rust
pub struct Foo {
    bar: Bar
}

pub struct Bar;

impl Foo {
    // ...
}

impl Bar {
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

## Naming Quantities

Quantities of something should be named `foo_count` where `foo` is the thing you're quantifying. It would be incorrect
to name this variable `num_foos`.

## Documentation

All public items should be documented. Documentation must be written with complete sentences and correct grammar.
Consider using [intra-doc links](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
where appropriate.
