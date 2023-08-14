use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CRAFT_REQUEST_C2S)]
pub struct CraftRequestC2s<'a> {
    pub window_id: i8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}
