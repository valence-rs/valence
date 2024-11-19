use std::{borrow::Cow, io::Write};

use anyhow::Error;
use valence_ident::Ident;
use valence_math::{DVec3, Vec3};

use crate::{Decode, Encode, Packet, Particle, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplodeS2c<'a> {
    pub pos: DVec3,
    pub player_motion: Option<Vec3>,
    pub particle: Particle,
    pub sound: ExplosionSound<'a>,
    //TODO: particle data and sound
}

#[derive(Clone, Debug)]
pub struct ExplosionSound<'a> {
    pub id: VarInt,
    // only present if id is 0
    pub name: Option<Ident<Cow<'a, str>>>,
    // only present if id is 0 And optional
    pub range: Option<f32>,
}

impl<'a> Decode<'a> for ExplosionSound<'a> {
    fn decode(buffer: &mut &'a [u8]) -> Result<Self, Error> {
        let id = VarInt::decode(buffer)?;
        if id.0 == 0 {
            let name = Ident::decode(buffer)?;
            let range = Option::<f32>::decode(buffer)?;
            Ok(Self {
                id,
                name: Some(name),
                range,
            })
        } else {
            Ok(Self {
                id,
                name: None,
                range: None,
            })
        }
    }
}

impl<'a> Encode for ExplosionSound<'a> {
    fn encode(&self, mut buffer: impl Write) -> Result<(), Error> {
        self.id.encode(&mut buffer)?;
        if self.id.0 == 0 {
            self.name.as_ref().unwrap().encode(&mut buffer)?;
            self.range.encode(&mut buffer)?;
        }
        Ok(())
    }
}
