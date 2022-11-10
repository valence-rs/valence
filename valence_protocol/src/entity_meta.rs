//! Types used in the entity metadata packet.

use crate::{Decode, Encode};

/// Represents an optional `u32` value excluding [`u32::MAX`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Encode, Decode)]
pub struct EulerAngle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Encode, Decode)]
pub enum Facing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Encode, Decode)]
pub struct VillagerData {
    pub kind: VillagerKind,
    pub profession: VillagerProfession,
    pub level: i32,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum BoatKind {
    #[default]
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum FrogKind {
    #[default]
    Temperate,
    Warm,
    Cold,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Encode, Decode)]
pub enum PaintingKind {
    #[default]
    Kebab,
    Aztec,
    Alban,
    Aztec2,
    Bomb,
    Plant,
    Wasteland,
    Pool,
    Courbet,
    Sea,
    Sunset,
    Creebet,
    Wanderer,
    Graham,
    Match,
    Bust,
    Stage,
    Void,
    SkullAndRoses,
    Wither,
    Fighters,
    Pointer,
    Pigscene,
    BurningSkull,
    Skeleton,
    Earth,
    Wind,
    Water,
    Fire,
    DonkeyKong,
}
