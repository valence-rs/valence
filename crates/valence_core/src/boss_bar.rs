use bitfield_struct::bitfield;
use valence_core_macros::{Decode, Encode};

/// The color of a boss bar.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

/// The division of a boss bar.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarDivision {
    NoDivision,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

/// The flags of a boss bar (darken sky, dragon bar, create fog).
#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct BossBarFlags {
    pub darken_sky: bool,
    pub dragon_bar: bool,
    pub create_fog: bool,
    #[bits(5)]
    _pad: u8,
}
