use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
}
