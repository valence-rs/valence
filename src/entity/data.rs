#![allow(clippy::all, missing_docs)]

use crate::block::{BlockPos, BlockState};
use crate::entity::meta::*;
use crate::entity::EntityId;
use crate::protocol::{Encode, VarInt};
use crate::text::Text;
use crate::uuid::Uuid;

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
