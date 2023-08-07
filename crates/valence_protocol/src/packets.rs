use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use uuid::Uuid;
use valence_generated::packet_id;

use crate::ident::Ident;
use crate::property::Property;
use crate::raw::RawBytes;
use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode, Packet, PacketState};

pub mod handshaking;
pub mod login;
pub mod play;
pub mod status;
