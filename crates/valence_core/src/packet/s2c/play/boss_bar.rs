use bitfield_struct::bitfield;
use uuid::Uuid;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct BossBarS2c {
    pub id: Uuid,
    pub action: Action,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Action {
    Add {
        title: Text,
        health: f32,
        color: Color,
        division: Division,
        flags: Flags,
    },
    Remove,
    UpdateHealth(f32),
    UpdateTitle(Text),
    UpdateStyle(Color, Division),
    UpdateFlags(Flags),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Color {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Division {
    NoDivision,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}
