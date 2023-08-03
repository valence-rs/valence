use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCOREBOARD_OBJECTIVE_UPDATE_S2C)]
pub struct ScoreboardObjectiveUpdateS2c<'a> {
    pub objective_name: &'a str,
    pub mode: ObjectiveMode,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum ObjectiveMode {
    Create {
        objective_display_name: Text,
        render_type: ObjectiveRenderType,
    },
    Remove,
    Update {
        objective_display_name: Text,
        render_type: ObjectiveRenderType,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ObjectiveRenderType {
    Integer,
    Hearts,
}
