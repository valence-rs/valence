use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateAttributesS2c<'a> {
    pub entity_id: VarInt,
    pub properties: Vec<AttributeProperty<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeProperty<'a> {
    pub id: VarInt, // This could be an enum, but seems like arbitray values are supported, while a
    // few are special:w
    pub value: f64,
    pub modifiers: Vec<AttributeModifier<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeModifier<'a> {
    pub id: Ident<Cow<'a, str>>,
    pub amount: f64,
    pub operation: u8,
}
