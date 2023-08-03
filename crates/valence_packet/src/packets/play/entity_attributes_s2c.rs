use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ATTRIBUTES_S2C)]
pub struct EntityAttributesS2c<'a> {
    pub entity_id: VarInt,
    pub properties: Vec<AttributeProperty<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeProperty<'a> {
    pub key: Ident<Cow<'a, str>>,
    pub value: f64,
    pub modifiers: Vec<AttributeModifier>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeModifier {
    pub uuid: Uuid,
    pub amount: f64,
    pub operation: u8,
}
