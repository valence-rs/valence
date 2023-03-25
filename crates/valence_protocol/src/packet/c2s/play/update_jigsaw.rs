use std::borrow::Cow;

use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct UpdateJigsawC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<Cow<'a, str>>,
    pub target: Ident<Cow<'a, str>>,
    pub pool: Ident<Cow<'a, str>>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
}
