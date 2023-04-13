use std::fmt;
use std::ops::Deref;

use bevy_app::{CoreSet, Plugin};
/// Contains shared components and world queries.
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
use uuid::Uuid;
use valence_protocol::types::{GameMode as ProtocolGameMode, Property};

use crate::client::FlushPacketsSet;
use crate::util::{from_yaw_and_pitch, to_yaw_and_pitch, is_valid_username};
use crate::view::ChunkPos;

pub(crate) struct ComponentPlugin;

impl Plugin for ComponentPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            (update_old_position, update_old_location)
                .in_base_set(CoreSet::PostUpdate)
                .after(FlushPacketsSet),
        )
        .add_system(despawn_marked_entities.in_base_set(CoreSet::Last));
    }
}

fn update_old_position(mut query: Query<(&Position, &mut OldPosition)>) {
    for (pos, mut old_pos) in &mut query {
        old_pos.0 = pos.0;
    }
}

fn update_old_location(mut query: Query<(&Location, &mut OldLocation)>) {
    for (loc, mut old_loc) in &mut query {
        old_loc.0 = loc.0;
    }
}

/// Despawns all the entities marked as despawned with the [`Despawned`]
/// component.
fn despawn_marked_entities(mut commands: Commands, entities: Query<Entity, With<Despawned>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

/// A [`Component`] for marking entities that should be despawned at the end of
/// the tick.
///
/// In Valence, some entities such as [Minecraft entities] are not allowed to
/// be removed from the [`World`] directly. Instead, you must give the entities
/// you wish to despawn the `Despawned` component. At the end of the tick,
/// Valence will despawn all entities with this component for you.
///
/// It is legal to remove components or delete entities that Valence does not
/// know about at any time.
///
/// [Minecraft entities]: crate::entity
#[derive(Component, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Despawned;

/// The universally unique identifier of an entity. Component wrapper for a
/// [`Uuid`].
///
/// This component is expected to remain _unique_ and _constant_ during the
/// lifetime of the entity. The [`Default`] impl generates a new random UUID.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct UniqueId(pub Uuid);

/// Generates a new random UUID.
impl Default for UniqueId {
    fn default() -> Self {
        Self(Uuid::from_bytes(rand::random()))
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct Username(pub String);

impl Username {
    pub fn is_valid(&self) -> bool {
        is_valid_username(&self.0)
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
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

impl From<Vec<Property>> for Properties {
    fn from(value: Vec<Property>) -> Self {
        Self(value)
    }
}

impl Deref for Properties {
    type Target = [Property];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]

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
        Self(Entity::PLACEHOLDER)
    }
}

impl PartialEq<OldLocation> for Location {
    fn eq(&self, other: &OldLocation) -> bool {
        self.0 == other.0
    }
}

/// The value of [`Location`] from the end of the previous tick.
///
/// **NOTE**: You should not modify this component after the entity is spawned.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct OldLocation(Entity);

impl OldLocation {
    pub fn new(instance: Entity) -> Self {
        Self(instance)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

impl Default for OldLocation {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

impl PartialEq<Location> for OldLocation {
    fn eq(&self, other: &Location) -> bool {
        self.0 == other.0
    }
}

#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Position(pub DVec3);

impl Position {
    pub fn new(pos: impl Into<DVec3>) -> Self {
        Self(pos.into())
    }

    pub fn chunk_pos(&self) -> ChunkPos {
        ChunkPos::from_dvec3(self.0)
    }

    pub fn get(self) -> DVec3 {
        self.0
    }

    pub fn set(&mut self, pos: impl Into<DVec3>) {
        self.0 = pos.into();
    }
}

impl PartialEq<OldPosition> for Position {
    fn eq(&self, other: &OldPosition) -> bool {
        self.0 == other.0
    }
}

/// The value of [`Location`] from the end of the previous tick.
///
/// **NOTE**: You should not modify this component after the entity is spawned.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct OldPosition(DVec3);

impl OldPosition {
    pub fn new(pos: impl Into<DVec3>) -> Self {
        Self(pos.into())
    }

    pub fn get(self) -> DVec3 {
        self.0
    }

    pub fn chunk_pos(self) -> ChunkPos {
        ChunkPos::from_dvec3(self.0)
    }
}

impl PartialEq<Position> for OldPosition {
    fn eq(&self, other: &Position) -> bool {
        self.0 == other.0
    }
}

/// Describes the direction an entity is looking using pitch and yaw angles.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct Look {
    /// The yaw angle in degrees.
    pub yaw: f32,
    /// The pitch angle in degrees.
    pub pitch: f32,
}

impl Look {
    pub const fn new(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }

    /// Gets a normalized direction vector from the yaw and pitch.
    pub fn vec(&self) -> Vec3 {
        from_yaw_and_pitch(self.yaw, self.pitch)
    }

    /// Sets the yaw and pitch using a normalized direction vector.
    pub fn set_vec(&mut self, vec: Vec3) {
        (self.yaw, self.pitch) = to_yaw_and_pitch(vec);
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct OnGround(pub bool);

/// General-purpose reusable byte buffer.
///
/// No guarantees are made about the buffer's contents between systems.
/// Therefore, the inner `Vec` should be cleared before use.
#[derive(Component, Default, Debug)]
pub struct ScratchBuf(pub Vec<u8>);
