use std::io::Write;

use tracing::warn;
use valence_protocol::encoder::{encode_packet, encode_packet_compressed, PacketEncoder};
use valence_protocol::Packet;

