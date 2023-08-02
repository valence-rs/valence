use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use uuid::Uuid;
use valence_core::ident::Ident;
use valence_core::property::Property;
use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Decode, Encode};
use valence_core::text::Text;

use crate::protocol::{packet_id, Packet, PacketState};

pub mod handshaking;
pub mod login;
pub mod play;
pub mod status;
