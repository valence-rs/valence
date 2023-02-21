use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ExplosionS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<&'a str>,
    pub make_all: bool,
}
