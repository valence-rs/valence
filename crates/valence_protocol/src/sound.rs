use std::borrow::Cow;

pub use valence_generated::sound::Sound;
use valence_ident::Ident;

use crate::id_or::IdOr;
use crate::{Decode, Encode};

pub type SoundId<'a> = IdOr<'a, SoundDirect<'a>>;

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
pub struct SoundDirect<'a> {
    pub id: Ident<Cow<'a, str>>,
    pub range: Option<f32>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}
