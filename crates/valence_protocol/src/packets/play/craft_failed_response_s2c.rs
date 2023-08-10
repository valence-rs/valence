use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CRAFT_FAILED_RESPONSE_S2C)]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
}
