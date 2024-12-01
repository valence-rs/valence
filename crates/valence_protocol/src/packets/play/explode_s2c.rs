use std::{borrow::Cow, io::Write};

use anyhow::Error;
use valence_ident::Ident;
use valence_math::{DVec3, Vec3};

use crate::{sound::SoundId, Decode, Encode, Packet, Particle, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplodeS2c<'a> {
    pub pos: DVec3,
    pub player_motion: Option<Vec3>,
    pub particle: Particle<'a>,
    pub sound: SoundId<'a>,
}
