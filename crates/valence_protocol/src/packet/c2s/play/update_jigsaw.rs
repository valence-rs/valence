use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateJigsawC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<&'a str>,
    pub target: Ident<&'a str>,
    pub pool: Ident<&'a str>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
}
