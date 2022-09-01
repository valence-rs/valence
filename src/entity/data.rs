//! Contains the [`TrackedData`] and the types for each variant.

#![allow(clippy::all, missing_docs, trivial_numeric_casts)]

use crate::block::{BlockPos, BlockState};
use crate::entity::types::*;
use crate::protocol::{Encode, VarInt};
use crate::text::Text;
use crate::uuid::Uuid;

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
