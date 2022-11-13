//! Contains the [`TrackedData`] and the types for each variant.

#![allow(clippy::all, missing_docs, trivial_numeric_casts)]

use uuid::Uuid;
use valence_protocol::block::BlockState;
use valence_protocol::block_pos::BlockPos;
use valence_protocol::entity_meta::*;
use valence_protocol::text::Text;
use valence_protocol::var_int::VarInt;
use valence_protocol::Encode;

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
