use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct HandshakeC2s<'a> {
    pub protocol_version: VarInt,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum NextState {
    #[packet(tag = 1)]
    Status,
    #[packet(tag = 2)]
    Login,
}
