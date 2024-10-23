use std::borrow::Cow;

use valence_ident::Ident;

use crate::{BlockPos, Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetJigsawBlockC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<Cow<'a, str>>,
    pub target: Ident<Cow<'a, str>>,
    pub pool: Ident<Cow<'a, str>>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
    pub selection_priority: VarInt,
    pub placement_priority: VarInt,
}
