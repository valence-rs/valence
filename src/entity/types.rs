//! Contains a struct for each variant in [`EntityKind`].

#![allow(clippy::all, missing_docs)]

use crate::block::{BlockPos, BlockState};
use crate::entity::data::*;
use crate::entity::EntityId;
use crate::protocol_inner::{Encode, VarInt};
use crate::text::Text;
use crate::uuid::Uuid;

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
