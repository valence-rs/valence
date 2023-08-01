use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SYNCHRONIZE_RECIPES_S2C)]
pub struct SynchronizeRecipesS2c<'a> {
    // TODO: this should be a Vec<Recipe<'a>>
    pub recipes: RawBytes<'a>,
}
