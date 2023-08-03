use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SELECT_ADVANCEMENT_TAB_S2C)]
pub struct SelectAdvancementTabS2c<'a> {
    pub identifier: Option<Ident<Cow<'a, str>>>,
}
