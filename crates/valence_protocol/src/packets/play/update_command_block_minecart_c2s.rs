use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateCommandBlockMinecartC2s<'a> {
    pub entity_id: VarInt,
    pub command: &'a str,
    pub track_output: bool,
}
