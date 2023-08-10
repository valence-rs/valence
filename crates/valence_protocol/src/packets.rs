//! All of Minecraft's network packets.
//!
//! Packets are grouped in submodules according to the protocol stage they're
//! used in. Names are derived from the FabricMC Yarn mappings for consistency.

use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use uuid::Uuid;
use valence_generated::packet_id;

use crate::property::Property;
use crate::raw::RawBytes;
use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode, Ident, Packet, PacketState};

pub mod handshaking;
pub mod login;
pub mod play;
pub mod status;
