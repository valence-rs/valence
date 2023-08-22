use crate::{Bounded, Decode, Encode, Packet, VarInt};

pub const MAX_TITLE_CHARS: usize = 128;
pub const MAX_PAGE_CHARS: usize = 8192;
pub const MAX_PAGES: usize = 200;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct BookUpdateC2s<'a> {
    pub slot: VarInt,
    pub entries: Bounded<Vec<Bounded<&'a str, MAX_PAGE_CHARS>>, MAX_PAGES>,
    pub title: Option<Bounded<&'a str, MAX_TITLE_CHARS>>,
}
