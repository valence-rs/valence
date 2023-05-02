use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct BookUpdateC2s<'a> {
    pub slot: VarInt,
    pub entries: Vec<&'a str>,
    pub title: Option<&'a str>,
}
