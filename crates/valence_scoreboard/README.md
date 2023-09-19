# valence_scoreboard

This crate provides functionality for creating and managing scoreboards. In Minecraft, a scoreboard references an [`Objective`], which is a mapping from strings to scores. Typically, the string is a player name, and the score is a number of points, but the string can be any arbitrary string <= 40 chars, and the score can be any integer.

In Valence, scoreboards obey the rules implied by layers, meaning that every Objective must have an [`LayerId`] associated with it. Scoreboards are only transmitted to clients if the [`EntityLayer`] is visible to the client.

To create a scoreboard, spawn an [`ObjectiveBundle`]. The [`Objective`] component represents the identifier that the client uses to reference the scoreboard.

Example:

```rust
# use bevy_ecs::prelude::*;
use valence_scoreboard::*;
use valence_server::protocol::text::IntoText;

fn spawn_scoreboard(mut commands: Commands) {
	commands.spawn(ObjectiveBundle {
		name: Objective::new("foo"),
		display: ObjectiveDisplay("Foo".bold()),
		..Default::default()
	});
}
```
