use std::borrow::Cow;
use std::io::Write;

use crate::ident::Ident;
use crate::item::ItemStack;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};
use crate::text::Text;

pub type AdvancementUpdateS2c<'a> =
    GenericAdvancementUpdateS2c<'a, (Ident<Cow<'a, str>>, Advancement<'a>)>;

#[derive(Clone, Debug, Encode, Decode)]
pub struct GenericAdvancementUpdateS2c<'a, AM: 'a> {
    pub reset: bool,
    pub advancement_mapping: Vec<AM>,
    pub identifiers: Vec<Ident<Cow<'a, str>>>,
    pub progress_mapping: Vec<(Ident<Cow<'a, str>>, Vec<AdvancementCriteria<'a>>)>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct Advancement<'a> {
    pub parent_id: Option<Ident<Cow<'a, str>>>,
    pub display_data: Option<AdvancementDisplay<'a>>,
    pub criteria: Vec<(Ident<Cow<'a, str>>, ())>,
    pub requirements: Vec<AdvancementRequirements<'a>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct AdvancementRequirements<'a> {
    pub requirement: Vec<&'a str>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AdvancementDisplay<'a> {
    pub title: Cow<'a, Text>,
    pub description: Cow<'a, Text>,
    pub icon: Option<ItemStack>,
    pub frame_type: VarInt,
    pub flags: i32,
    pub background_texture: Option<Ident<Cow<'a, str>>>,
    pub x_coord: f32,
    pub y_coord: f32,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct AdvancementCriteria<'a> {
    pub criterion_identifier: Ident<Cow<'a, str>>,
    /// If present, the criteria has been achieved at the
    /// time wrapped; time represented as millis since epoch
    pub criterion_progress: Option<i64>,
}

impl Encode for AdvancementDisplay<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.title.encode(&mut w)?;
        self.description.encode(&mut w)?;
        self.icon.encode(&mut w)?;
        self.frame_type.encode(&mut w)?;
        self.flags.encode(&mut w)?;

        match self.background_texture.as_ref() {
            None => {}
            Some(texture) => texture.encode(&mut w)?,
        }

        self.x_coord.encode(&mut w)?;
        self.y_coord.encode(&mut w)?;

        Ok(())
    }
}

impl<'a> Decode<'a> for AdvancementDisplay<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let title = <Cow<'a, Text>>::decode(r)?;
        let description = <Cow<'a, Text>>::decode(r)?;
        let icon = Option::<ItemStack>::decode(r)?;
        let frame_type = VarInt::decode(r)?;
        let flags = i32::decode(r)?;

        let background_texture = if flags & 1 == 1 {
            Some(Ident::decode(r)?)
        } else {
            None
        };

        let x_coord = f32::decode(r)?;
        let y_coord = f32::decode(r)?;

        Ok(Self {
            title,
            description,
            icon,
            frame_type,
            flags,
            background_texture,
            x_coord,
            y_coord,
        })
    }
}
