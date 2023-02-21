use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x22]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<&'a str>,
}
