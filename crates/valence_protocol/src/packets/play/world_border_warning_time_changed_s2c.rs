use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_WARNING_TIME_CHANGED_S2C)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
