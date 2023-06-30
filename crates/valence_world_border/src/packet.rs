use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_CENTER_CHANGED_S2C)]
pub struct WorldBorderCenterChangedS2c {
    pub x_pos: f64,
    pub z_pos: f64,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_INITIALIZE_S2C)]
pub struct WorldBorderInitializeS2c {
    pub x: f64,
    pub z: f64,
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
    pub portal_teleport_boundary: VarInt,
    pub warning_blocks: VarInt,
    pub warning_time: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_INTERPOLATE_SIZE_S2C)]
pub struct WorldBorderInterpolateSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_SIZE_CHANGED_S2C)]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_WARNING_BLOCKS_CHANGED_S2C)]
pub struct WorldBorderWarningBlocksChangedS2c {
    pub warning_blocks: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_WARNING_TIME_CHANGED_S2C)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
