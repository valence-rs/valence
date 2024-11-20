use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct RecipeBookRemoveS2c {
    pub recipes: Vec<VarInt>,
}
