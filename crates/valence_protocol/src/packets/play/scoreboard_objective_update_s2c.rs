use bevy_ecs::prelude::*;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCOREBOARD_OBJECTIVE_UPDATE_S2C)]
pub struct ScoreboardObjectiveUpdateS2c<'a> {
    pub objective_name: &'a str,
    pub mode: ObjectiveMode<'a>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum ObjectiveMode<'a> {
    Create {
        objective_display_name: Cow<'a, Text>,
        render_type: ObjectiveRenderType,
    },
    Remove,
    Update {
        objective_display_name: Cow<'a, Text>,
        render_type: ObjectiveRenderType,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Component, Default)]
pub enum ObjectiveRenderType {
    /// Display the value as a number.
    #[default]
    Integer,
    /// Display the value as hearts.
    Hearts,
}
