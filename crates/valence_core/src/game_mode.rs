use bevy_ecs::prelude::*;

use crate::protocol::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Encode, Decode, Component)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}
