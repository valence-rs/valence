use std::borrow::Cow;

use uuid::Uuid;
use valence_core::ident::Ident;
use valence_core::property::Property;
use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet, PacketState};
use valence_core::text::Text;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::HANDSHAKE_C2S, state = PacketState::Handshaking)]
pub struct HandshakeC2s<'a> {
    pub protocol_version: VarInt,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum HandshakeNextState {
    #[packet(tag = 1)]
    Status,
    #[packet(tag = 2)]
    Login,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_PING_C2S, state = PacketState::Status)]
pub struct QueryPingC2s {
    pub payload: u64,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_REQUEST_C2S, state = PacketState::Status)]
pub struct QueryRequestC2s;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_PONG_S2C, state = PacketState::Status)]
pub struct QueryPongS2c {
    pub payload: u64,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_RESPONSE_S2C, state = PacketState::Status)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_HELLO_C2S, state = PacketState::Login)]
pub struct LoginHelloC2s<'a> {
    pub username: &'a str, // TODO: bound this
    pub profile_id: Option<Uuid>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_KEY_C2S, state = PacketState::Login)]
pub struct LoginKeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_QUERY_RESPONSE_C2S, state = PacketState::Login)]
pub struct LoginQueryResponseC2s<'a> {
    pub message_id: VarInt,
    pub data: Option<RawBytes<'a>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_COMPRESSION_S2C, state = PacketState::Login)]
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_DISCONNECT_S2C, state = PacketState::Login)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_HELLO_S2C, state = PacketState::Login)]
pub struct LoginHelloS2c<'a> {
    pub server_id: &'a str,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_QUERY_REQUEST_S2C, state = PacketState::Login)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_SUCCESS_S2C, state = PacketState::Login)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: &'a str, // TODO: bound this.
    pub properties: Cow<'a, [Property]>,
}
