use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_CENTER_CHANGED_S2C)]
pub struct WorldBorderCenterChangedS2c {
    pub x_pos: f64,
    pub z_pos: f64,
}
