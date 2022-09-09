//! Primitive types used in getters and setters on entities.

use std::io::Write;

use crate::protocol::{Decode, Encode, VarInt};

/// Represents an optional `u32` value excluding [`u32::MAX`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct OptionalInt(u32);

impl OptionalInt {
    /// Returns `None` iff `n` is Some(u32::MAX).
    pub fn new(n: impl Into<Option<u32>>) -> Option<Self> {
        match n.into() {
            None => Some(Self(0)),
            Some(u32::MAX) => None,
            Some(n) => Some(Self(n + 1)),
        }
    }

    pub fn get(self) -> Option<u32> {
        self.0.checked_sub(1)
    }
}

impl Encode for OptionalInt {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(self.0 as i32).encode(w)
    }
}

impl Decode for OptionalInt {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(Self(VarInt::decode(r)?.0 as u32))
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct EulerAngle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl EulerAngle {
    pub fn new(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self { pitch, yaw, roll }
    }
}

impl Encode for EulerAngle {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.pitch.encode(w)?;
        self.yaw.encode(w)?;
        self.roll.encode(w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Facing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Encode for Facing {
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
pub enum MainArm {
    Left,
    #[default]
    Right,
}

impl Encode for MainArm {
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
    Kebab, // TODO
}

impl Encode for PaintingKind {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}

// TODO
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Particle {
    EntityEffect = 21,
}

impl Encode for Particle {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        VarInt(*self as i32).encode(w)
    }
}
