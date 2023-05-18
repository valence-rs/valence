use uuid::Uuid;
use valence_core::packet::var_int::VarInt;
use valence_core::packet::Packet;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = )]
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

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryPingC2s {
    pub payload: u64,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryRequestC2s;

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryPongS2c {
    pub payload: u64,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginHelloC2s<'a> {
    pub username: &'a str, // TODO: bound this
    pub profile_id: Option<Uuid>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginKeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginQueryResponseC2s<'a> {
    pub message_id: VarInt,
    pub data: Option<RawBytes<'a>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LoginHelloS2c<'a> {
    pub server_id: &'a str,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: &'a str, // TODO: bound this.
    pub properties: Cow<'a, [Property]>,
}
