use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SelectAdvancementTabS2c<'a> {
    pub identifier: Option<Ident<Cow<'a, str>>>,
}
