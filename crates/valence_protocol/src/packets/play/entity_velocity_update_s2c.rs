use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_VELOCITY_UPDATE_S2C)]
pub struct EntityVelocityUpdateS2c {
    pub entity_id: VarInt,
    pub velocity: Velocity,
}
