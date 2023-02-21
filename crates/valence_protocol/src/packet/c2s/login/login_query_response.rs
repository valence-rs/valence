use crate::raw_bytes::RawBytes;
use crate::var_int::VarInt;

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x02]
pub struct LoginQueryResponseC2s<'a> {
    pub message_id: VarInt,
    pub data: Option<RawBytes<'a>>,
}
