#![allow(missing_docs)]

use std::io::Write;

use crate::protocol::Encode;
use crate::var_int::VarInt;

#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug)]
pub struct ArmorStandRotations {
    /// Rotation on the X axis in degrees.
    pub x: f32,
    /// Rotation on the Y axis in degrees.
    pub y: f32,
    /// Rotation on the Z axis in degrees.
    pub z: f32,
}

impl ArmorStandRotations {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Encode for ArmorStandRotations {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.x.encode(w)?;
        self.y.encode(w)?;
        self.z.encode(w)
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
    Croaking,
    UsingTongue,
    Roaring,
    Sniffing,
    Emerging,
    Digging,
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

/// The main hand of a player.
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum CatVariant {
    Tabby,
    Black,
    Red,
    Siamese,
    BritishShorthair,
    Calico,
    Persian,
    Ragdoll,
    White,
    Jellie,
    AllBlack,
}

impl Default for CatVariant {
    fn default() -> Self {
        CatVariant::Black
    }
}

impl Encode for CatVariant {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum FrogVariant {
    Temperate,
    Warm,
    Cold,
}

impl Default for FrogVariant {
    fn default() -> Self {
        FrogVariant::Temperate
    }
}

impl Encode for FrogVariant {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PaintingVariant {
    Default, // TODO
}

impl Default for PaintingVariant {
    fn default() -> Self {
        PaintingVariant::Default
    }
}

impl Encode for PaintingVariant {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}
