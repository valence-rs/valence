Simplify the creation of Valence commands with a derive macro.

## Usage

```rust
#[derive(Command, Debug, Clone)]
#[paths("teleport", "tp")]
#[scopes("valence.command.teleport")]
enum TeleportCommand {
    #[paths = "{location}"]
    ExecutorToLocation { location: Vec3Parser },
    #[paths = "{target}"]
    ExecutorToTarget { target: EntitySelector },
    #[paths = "{from} {to}"]
    TargetToTarget {
        from: EntitySelector,
        to: EntitySelector,
    },
    #[paths = "{target} {location}"]
    TargetToLocation {
        target: EntitySelector,
        location: Vec3Parser,
    },
}

#[derive(Command, Debug, Clone)]
#[paths("gamemode", "gm")]
#[scopes("valence.command.gamemode")]
enum GamemodeCommand {
    #[paths("survival", "{/} gms")]
    Survival,
    #[paths("creative", "{/} gmc")]
    Creative,
    #[paths("adventure", "{/} gma")]
    Adventure,
    #[paths("spectator", "{/} gmsp")]
    Spectator,
}

#[derive(Command, Debug, Clone)]
#[paths("test", "t")]
#[scopes("valence.command.test")]
#[allow(dead_code)]
enum TestCommand {
    // 3 literals with an arg each
    #[paths("a {a} b {b} c {c}", "{a} {b} {c}")]
    A { a: String, b: i32, c: f32 },
    // 2 literals with an arg last being optional (Because of the greedy string before the end
    // this is technically unreachable)
    #[paths = "a {a} {b} b {c?}"]
    B {
        a: Vec3Parser,
        b: GreedyString,
        c: Option<String>,
    },
    // greedy string optional arg
    #[paths = "a {a} b {b?}"]
    C { a: String, b: Option<GreedyString> },
    // greedy string required arg
    #[paths = "a {a} b {b}"]
    D { a: String, b: GreedyString },
    // five optional args and an ending greedyString
    #[paths("options {a?} {b?} {c?} {d?} {e?}", "options {b?} {a?} {d?} {c?} {e?}")]
    E {
        a: Option<i32>,
        b: Option<QuotableString>,
        c: Option<Vec2Parser>,
        d: Option<Vec3Parser>,
        e: Option<GreedyString>,
    },
}
```

## Attributes

### `#[paths(...)]` or `#[paths = "..."]`

The `#[paths(...)]` or `#[paths = "..."]` attribute is used to specify the different paths that can be used to invoke
the command. The paths are specified as string literals, where any arguments are enclosed in curly braces `{}`.
The arguments are then mapped to fields in the command enum variant.

For example, in the `Teleport` enum, the `ExecutorToLocation` variant has a path of `{location}`, which means it expects
a single argument called `location` of type `Vec3Parser`. The `ExecutorToTarget` variant has a path of `{target}`, which
expects a single argument called `target` of type `EntitySelector`.

The paths attribute can have multiple values separated by commas, representing alternative paths that can be used to 
invoke the command. These alternative paths can have different argument orders, but they must have the same arguments.

Their are two special paths that can be used. The first is `{/}`, which represents the root command, this can only be 
used at the start of the command to specify it as a direct child of the root node. The second is `{<arg>?}`, which
represents an optional argument. The optional argument must only be followed by other optional arguments or the end of 
the path.

### `#[scopes(...)]` or `#[scopes = "..."]`

The `#[scopes(...)]` or `#[scopes = "..."]` attribute is used to specify the scopes that the command belongs to. Scopes
are used to specify who can use the command. The scopes are specified as string literals, where each scope is separated
by a colon.

For example, in the `Teleport` enum, the variants are assigned the scope `valence:command:teleport`, which means they
can be used by anyone with the `valence:command:teleport`, `valence:command` or `valence` scope.

The scopes attribute can have multiple values separated by commas, representing the different scopes that the command
belongs to.

## How do command graphs work anyway?

This is the core of the command system. It is a graph of `CommandNode`s that are connected by the `CommandEdgeType`. The
graph is used to determine what command to run when a command is entered. The graph is also used to generate the command
tree that is sent to the client. You can think of it as a tree where each leaf is part of a command, and the path to the
leaf is the command. See the documentation for `command.rs` in `valence_command` for more information.


### Our teleport command from the example (made with graphviz)
```text
                                              ┌────────────────────────────────┐
                                              │              Root              │ ─┐
                                              └────────────────────────────────┘  │
                                                │                                 │
                                                │ Child                           │
                                                ▼                                 │
                                              ┌────────────────────────────────┐  │
                                              │          Literal: tp           │  │
                                              └────────────────────────────────┘  │
                                                │                                 │
                                                │ Redirect                        │ Child
                                                ▼                                 ▼
┌──────────────────────────────────┐  Child   ┌──────────────────────────────────────────────────────────────────────────────┐
│  Argument: <destination:entity>  │ ◀─────── │                              Literal: teleport                               │
└──────────────────────────────────┘          └──────────────────────────────────────────────────────────────────────────────┘
                                                │                                           │
                                                │ Child                                     │ Child
                                                ▼                                           ▼
┌──────────────────────────────────┐  Child   ┌────────────────────────────────┐          ┌──────────────────────────────────┐
│ Argument: <destination:location> │ ◀─────── │   Argument: <target:entity>    │          │ Argument: <destination:location> │
└──────────────────────────────────┘          └────────────────────────────────┘          └──────────────────────────────────┘
                                                │
                                                │ Child
                                                ▼
                                              ┌────────────────────────────────┐
                                              │ Argument: <destination:entity> │
                                              └────────────────────────────────┘
```