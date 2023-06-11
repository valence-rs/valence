use std::borrow::Cow;

use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::protocol::{Decode, Encode};

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct GlobalPos<'a> {
    pub dimension_name: Ident<Cow<'a, str>>,
    pub position: BlockPos,
}
