//! All of Minecraft's network packets.
//!
//! Packets are grouped in submodules according to the protocol stage they're
//! used in. Names are derived from the FabricMC Yarn mappings for consistency.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::Write;

use anyhow::{bail, ensure};
use uuid::Uuid;
use valence_generated::block::{BlockEntityKind, BlockKind, BlockState};
use valence_generated::packet_id;

use crate::game_mode::OptGameMode;
use crate::profile::PropertyValue;
use crate::raw::RawBytes;
use crate::text::Text;
use crate::var_int::VarInt;
use crate::{
    Bounded, Decode, Encode, FixedBitSet, Ident, ItemKind, ItemStack, Packet, PacketState, Velocity,
};

pub mod handshaking;
pub mod login;
pub mod play;
pub mod status;
