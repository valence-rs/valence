use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_SIZE_CHANGED_S2C)]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}
