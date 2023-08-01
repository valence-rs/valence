use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use uuid::Uuid;
use valence_core::ident::Ident;
use valence_core::property::Property;
use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_core::text::Text;

pub mod handshaking;
pub mod login;
pub mod play;
pub mod status;
