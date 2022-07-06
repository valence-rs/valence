#![allow(missing_docs)]

use std::io::Write;

use crate::protocol::{Encode, VarInt};

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
    pub kind: VillagerKind,
    pub profession: VillagerProfession,
    pub level: i32,
}

impl VillagerData {
    pub const fn new(kind: VillagerKind, profession: VillagerProfession, level: i32) -> Self {
        Self {
            kind,
            profession,
            level,
        }
    }
}

impl Default for VillagerData {
    fn default() -> Self {
        Self {
            kind: Default::default(),
            profession: Default::default(),
            level: 1,
        }
    }
}

impl Encode for VillagerData {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(self.kind as i32).encode(w)?;
        VarInt(self.profession as i32).encode(w)?;
        VarInt(self.level).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum VillagerKind {
    Desert,
    Jungle,
    #[default]
    Plains,
    Savanna,
    Snow,
    Swamp,
    Taiga,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum VillagerProfession {
    #[default]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum Pose {
    #[default]
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

impl Encode for Pose {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

/// The main hand of a player.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum MainHand {
    Left,
    #[default]
    Right,
}

impl Encode for MainHand {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        (*self as u8).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum BoatKind {
    #[default]
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

impl Encode for BoatKind {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum CatKind {
    Tabby,
    #[default]
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

impl Encode for CatKind {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum FrogKind {
    #[default]
    Temperate,
    Warm,
    Cold,
}

impl Encode for FrogKind {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum PaintingKind {
    #[default]
    Default, // TODO
}

impl Encode for PaintingKind {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}
