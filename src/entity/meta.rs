use std::io::Write;

use anyhow::Context;
use uuid::Uuid;

use crate::block_pos::BlockPos;
use crate::protocol::Encode;
use crate::var_int::VarInt;
use crate::{def_bitfield, Text};

#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug)]
pub struct ArmorStandRotations {
    pub x_degrees: f32,
    pub y_degrees: f32,
    pub z_degrees: f32,
}

impl ArmorStandRotations {
    pub fn new(x_degrees: f32, y_degrees: f32, z_degrees: f32) -> Self {
        Self {
            x_degrees,
            y_degrees,
            z_degrees,
        }
    }
}

impl Encode for ArmorStandRotations {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.x_degrees.encode(w)?;
        self.y_degrees.encode(w)?;
        self.z_degrees.encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Encode for Direction {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VillagerData {
    pub typ: VillagerType,
    pub profession: VillagerProfession,
    pub level: i32,
}

impl VillagerData {
    pub const fn new(typ: VillagerType, profession: VillagerProfession, level: i32) -> Self {
        Self {
            typ,
            profession,
            level,
        }
    }
}

impl Default for VillagerData {
    fn default() -> Self {
        Self {
            typ: Default::default(),
            profession: Default::default(),
            level: 1,
        }
    }
}

impl Encode for VillagerData {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(self.typ as i32).encode(w)?;
        VarInt(self.profession as i32).encode(w)?;
        VarInt(self.level).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum VillagerType {
    Desert,
    Jungle,
    Plains,
    Savanna,
    Snow,
    Swamp,
    Taiga,
}

impl Default for VillagerType {
    fn default() -> Self {
        Self::Plains
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum VillagerProfession {
    None,
    Armorer,
    Butcher,
    Cartographer,
    Cleric,
    Farmer,
    Fisherman,
    Fletcher,
    Leatherworker,
    Librarian,
    Mason,
    Nitwit,
    Shepherd,
    Toolsmith,
    Weaponsmith,
}

impl Default for VillagerProfession {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OptVarInt(Option<i32>);

impl Encode for OptVarInt {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match self.0 {
            Some(n) => VarInt(
                n.checked_add(1)
                    .context("i32::MAX is unrepresentable as an optional VarInt")?,
            )
            .encode(w),
            None => VarInt(0).encode(w),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Pose {
    Standing,
    FallFlying,
    Sleeping,
    Swimming,
    SpinAttack,
    Sneaking,
    LongJumping,
    Dying,
}

impl Default for Pose {
    fn default() -> Self {
        Self::Standing
    }
}

impl Encode for Pose {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MainHand {
    Left,
    Right,
}

impl Default for MainHand {
    fn default() -> Self {
        Self::Right
    }
}

impl Encode for MainHand {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        (*self as u8).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BoatVariant {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

impl Default for BoatVariant {
    fn default() -> Self {
        Self::Oak
    }
}

impl Encode for BoatVariant {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}
