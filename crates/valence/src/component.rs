use std::fmt;

/// Contains shared components and world queries.
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use glam::{DVec3, Vec3};
use uuid::Uuid;
use valence_protocol::types::{GameMode as ProtocolGameMode, Property};

use crate::util::{from_yaw_and_pitch, to_yaw_and_pitch};
use crate::view::ChunkPos;
use crate::NULL_ENTITY;

/// A [`Component`] for marking entities that should be despawned at the end of
/// the tick.
///
/// In Valence, some built-in components such as [`McEntity`] are not allowed to
/// be removed from the [`World`] directly. Instead, you must give the entities
/// you wish to despawn the `Despawned` component. At the end of the tick,
/// Valence will despawn all entities with this component for you.
///
/// It is legal to remove components or delete entities that Valence does not
/// know about at any time.
///
/// [`McEntity`]: crate::entity::McEntity
#[derive(Component, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Despawned;

#[derive(Component, Default, Clone, PartialEq, Eq, Debug)]
pub struct UniqueId(pub Uuid);

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Username(pub String);

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Properties(pub Vec<Property>);

impl Properties {
    /// Finds the property with the name "textures".
    pub fn textures(&self) -> Option<&Property> {
        self.0.iter().find(|prop| prop.name == "textures")
    }

    /// Finds the property with the name "textures".
    pub fn textures_mut(&mut self) -> Option<&mut Property> {
        self.0.iter_mut().find(|prop| prop.name == "textures")
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Default)]

pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl From<GameMode> for ProtocolGameMode {
    fn from(gm: GameMode) -> Self {
        match gm {
            GameMode::Survival => ProtocolGameMode::Survival,
            GameMode::Creative => ProtocolGameMode::Creative,
            GameMode::Adventure => ProtocolGameMode::Adventure,
            GameMode::Spectator => ProtocolGameMode::Spectator,
        }
    }
}

impl From<ProtocolGameMode> for GameMode {
    fn from(gm: ProtocolGameMode) -> Self {
        match gm {
            ProtocolGameMode::Survival => GameMode::Survival,
            ProtocolGameMode::Creative => GameMode::Creative,
            ProtocolGameMode::Adventure => GameMode::Adventure,
            ProtocolGameMode::Spectator => GameMode::Spectator,
        }
    }
}

/// Delay measured in milliseconds. Negative values indicate absence.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Ping(pub i32);

impl Default for Ping {
    fn default() -> Self {
        Self(-1)
    }
}

/// Contains the [`Instance`] an entity is located in. For the coordinates
/// within the instance, see [`Position`].
///
/// [`Instance`]: crate::instance::Instance
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct Location(pub Entity);

impl Default for Location {
    fn default() -> Self {
        Self(NULL_ENTITY)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct OldLocation(Entity);

impl OldLocation {
    pub(crate) fn update(mut query: Query<(&Location, &mut OldLocation), Changed<OldLocation>>) {
        for (loc, mut old_loc) in &mut query {
            old_loc.0 = loc.0;
        }
    }

    pub fn new(instance: Entity) -> Self {
        Self(instance)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

impl Default for OldLocation {
    fn default() -> Self {
        Self(NULL_ENTITY)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Position(pub DVec3);

impl Position {
    pub fn chunk_pos(&self) -> ChunkPos {
        ChunkPos::from_dvec3(self.0)
    }

    pub fn get(&self) -> DVec3 {
        self.0
    }

    pub fn set(&mut self, pos: impl Into<DVec3>) {
        self.0 = pos.into();
    }
}

#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct OldPosition(DVec3);

impl OldPosition {
    pub(crate) fn update(mut query: Query<(&Position, &mut OldPosition), Changed<Position>>) {
        for (pos, mut old_pos) in &mut query {
            old_pos.0 = pos.0;
        }
    }

    pub fn new(pos: DVec3) -> Self {
        Self(pos)
    }

    pub fn get(&self) -> DVec3 {
        self.0
    }

    pub fn chunk_pos(&self) -> ChunkPos {
        ChunkPos::from_dvec3(self.0)
    }
}

/// Velocity in m/s.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Velocity(pub Vec3);

#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Yaw(pub f32);

#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Pitch(pub f32);

#[derive(WorldQuery, PartialEq, Debug)]
#[world_query(mutable)]
pub struct DirectionMut {
    pub yaw: &'static mut Yaw,
    pub pitch: &'static mut Pitch,
}

impl DirectionMutItem<'_> {
    pub fn vec(&self) -> Vec3 {
        from_yaw_and_pitch(self.yaw.0, self.pitch.0)
    }

    pub fn set_vec(&mut self, vec: Vec3) {
        let (yaw, pitch) = to_yaw_and_pitch(vec);

        self.yaw.0 = yaw;
        self.pitch.0 = pitch;
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.0
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.0
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw.0 = yaw;
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch.0 = pitch;
    }
}

pub use DirectionMutReadOnly as Direction;
pub use DirectionMutReadOnlyItem as DirectionItem;

impl DirectionItem<'_> {
    pub fn vec(&self) -> Vec3 {
        from_yaw_and_pitch(self.yaw.0, self.pitch.0)
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.0
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.0
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct OnGround(pub bool);
