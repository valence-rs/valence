//! Contains the [`TrackedData`] enum and the types for each variant.

// TODO: clean this up.
#![allow(clippy::all, missing_docs, trivial_numeric_casts, dead_code)]

use uuid::Uuid;
use valence_protocol::entity_meta::*;
use valence_protocol::{BlockPos, BlockState, Encode, Text, VarInt};

include!(concat!(env!("OUT_DIR"), "/entity.rs"));
