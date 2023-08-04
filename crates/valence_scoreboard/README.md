# valence_scoreboard

This crate provides functionality for creating and managing scoreboards. In Minecraft, a scoreboard references an [`Objective`], which is a mapping from entities to scores.

In Valence, scoreboards obey the rules implied by [`EntityLayer`]s, meaning that every Objective must have an [`EntityLayerId`] associated with it. Scoreboards are only transmitted to clients if the [`EntityLayer`] is visible to the client.

To create a scoreboard, spawn an [`ObjectiveBundle`]. The [`Objective`] component represents the identifier that the client uses to reference the scoreboard.