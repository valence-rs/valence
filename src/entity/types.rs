#![allow(clippy::all, missing_docs)]

use crate::block::BlockState;
use crate::entity::meta::*;
use crate::protocol::Encode;
use crate::var_int::VarInt;
use crate::{BlockPos, EntityId, Text, Uuid};

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
