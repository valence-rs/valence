use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DAMAGE_TILT_S2C)]
pub struct DamageTiltS2c {
    /// The ID of the entity taking damage.
    pub entity_id: VarInt,
    /// The direction the damage is coming from in relation to the entity.
    pub yaw: f32,
}
