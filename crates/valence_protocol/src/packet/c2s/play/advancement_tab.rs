use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x25]
pub enum AdvancementTabC2s<'a> {
    OpenedTab { tab_id: Ident<&'a str> },
    ClosedScreen,
}
