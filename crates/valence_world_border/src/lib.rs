#![doc = include_str!("../README.md")]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use valence_server::client::{Client, UpdateClientsSet, VisibleChunkLayer};
use valence_server::protocol::packets::play::{
    WorldBorderCenterChangedS2c, WorldBorderInitializeS2c, WorldBorderInterpolateSizeS2c,
    WorldBorderSizeChangedS2c, WorldBorderWarningBlocksChangedS2c,
    WorldBorderWarningTimeChangedS2c,
};
use valence_server::protocol::WritePacket;
use valence_server::{ChunkLayer, Server};

// https://minecraft.wiki/w/World_border
pub const DEFAULT_PORTAL_LIMIT: i32 = 29999984;
pub const DEFAULT_DIAMETER: f64 = (DEFAULT_PORTAL_LIMIT * 2) as f64;
pub const DEFAULT_WARN_TIME: i32 = 15;
pub const DEFAULT_WARN_BLOCKS: i32 = 5;

pub struct WorldBorderPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWorldBorderSet;

impl Plugin for WorldBorderPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(PostUpdate, UpdateWorldBorderSet.before(UpdateClientsSet))
            .add_systems(
                PostUpdate,
                (
                    init_world_border_for_new_clients,
                    tick_world_border_lerp,
                    change_world_border_center,
                    change_world_border_warning_blocks,
                    change_world_border_warning_time,
                    change_world_border_portal_tp_boundary,
                )
                    .in_set(UpdateWorldBorderSet),
            );
    }
}

/// A bundle containing necessary components to enable world border
/// functionality. Add this to an entity with the [`ChunkLayer`] component.
#[derive(Bundle, Default, Debug)]
pub struct WorldBorderBundle {
    pub center: WorldBorderCenter,
    pub lerp: WorldBorderLerp,
    pub portal_teleport_boundary: WorldBorderPortalTpBoundary,
    pub warn_time: WorldBorderWarnTime,
    pub warn_blocks: WorldBorderWarnBlocks,
}

#[derive(Component, Default, Copy, Clone, PartialEq, Debug)]
pub struct WorldBorderCenter {
    pub x: f64,
    pub z: f64,
}

impl WorldBorderCenter {
    pub fn set(&mut self, x: f64, z: f64) {
        self.x = x;
        self.z = z;
    }
}

/// Component containing information to linearly interpolate the world border.
/// Contains the world border's diameter.
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldBorderLerp {
    /// The current diameter of the world border. This is updated automatically
    /// as the remaining ticks count down.
    pub current_diameter: f64,
    /// The desired diameter of the world border after lerping has finished.
    /// Modify this if you want to change the world border diameter.
    pub target_diameter: f64,
    /// Server ticks until the target diameter is reached. This counts down
    /// automatically.
    pub remaining_ticks: u64,
}
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deref, DerefMut)]
pub struct WorldBorderWarnTime(pub i32);

impl Default for WorldBorderWarnTime {
    fn default() -> Self {
        Self(DEFAULT_WARN_TIME)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deref, DerefMut)]
pub struct WorldBorderWarnBlocks(pub i32);

impl Default for WorldBorderWarnBlocks {
    fn default() -> Self {
        Self(DEFAULT_WARN_BLOCKS)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deref, DerefMut)]
pub struct WorldBorderPortalTpBoundary(pub i32);

impl Default for WorldBorderPortalTpBoundary {
    fn default() -> Self {
        Self(DEFAULT_PORTAL_LIMIT)
    }
}

impl Default for WorldBorderLerp {
    fn default() -> Self {
        Self {
            current_diameter: DEFAULT_DIAMETER,
            target_diameter: DEFAULT_DIAMETER,
            remaining_ticks: 0,
        }
    }
}

fn init_world_border_for_new_clients(
    mut clients: Query<(&mut Client, &VisibleChunkLayer), Changed<VisibleChunkLayer>>,
    wbs: Query<(
        &WorldBorderCenter,
        &WorldBorderLerp,
        &WorldBorderPortalTpBoundary,
        &WorldBorderWarnTime,
        &WorldBorderWarnBlocks,
    )>,
    server: Res<Server>,
) {
    for (mut client, layer) in &mut clients {
        if let Ok((center, lerp, portal_tp_boundary, warn_time, warn_blocks)) = wbs.get(layer.0) {
            let millis = lerp.remaining_ticks as i64 * 1000 / server.tick_rate().get() as i64;

            client.write_packet(&WorldBorderInitializeS2c {
                x: center.x,
                z: center.z,
                old_diameter: lerp.current_diameter,
                new_diameter: lerp.target_diameter,
                duration_millis: millis.into(),
                portal_teleport_boundary: portal_tp_boundary.0.into(),
                warning_blocks: warn_blocks.0.into(),
                warning_time: warn_time.0.into(),
            });
        }
    }
}

fn tick_world_border_lerp(
    mut wbs: Query<(&mut ChunkLayer, &mut WorldBorderLerp)>,
    server: Res<Server>,
) {
    for (mut layer, mut lerp) in &mut wbs {
        if lerp.is_changed() {
            if lerp.remaining_ticks == 0 {
                layer.write_packet(&WorldBorderSizeChangedS2c {
                    diameter: lerp.target_diameter,
                });

                lerp.current_diameter = lerp.target_diameter;
            } else {
                let millis = lerp.remaining_ticks as i64 * 1000 / server.tick_rate().get() as i64;

                layer.write_packet(&WorldBorderInterpolateSizeS2c {
                    old_diameter: lerp.current_diameter,
                    new_diameter: lerp.target_diameter,
                    duration_millis: millis.into(),
                });
            }
        }

        if lerp.remaining_ticks > 0 {
            let diff = lerp.target_diameter - lerp.current_diameter;
            lerp.current_diameter += diff / lerp.remaining_ticks as f64;

            lerp.remaining_ticks -= 1;
        }
    }
}

fn change_world_border_center(
    mut wbs: Query<(&mut ChunkLayer, &WorldBorderCenter), Changed<WorldBorderCenter>>,
) {
    for (mut layer, center) in &mut wbs {
        layer.write_packet(&WorldBorderCenterChangedS2c {
            x_pos: center.x,
            z_pos: center.z,
        });
    }
}

fn change_world_border_warning_blocks(
    mut wbs: Query<(&mut ChunkLayer, &WorldBorderWarnBlocks), Changed<WorldBorderWarnBlocks>>,
) {
    for (mut layer, warn_blocks) in &mut wbs {
        layer.write_packet(&WorldBorderWarningBlocksChangedS2c {
            warning_blocks: warn_blocks.0.into(),
        });
    }
}

fn change_world_border_warning_time(
    mut wbs: Query<(&mut ChunkLayer, &WorldBorderWarnTime), Changed<WorldBorderWarnTime>>,
) {
    for (mut layer, warn_time) in &mut wbs {
        layer.write_packet(&WorldBorderWarningTimeChangedS2c {
            warning_time: warn_time.0.into(),
        });
    }
}

fn change_world_border_portal_tp_boundary(
    mut wbs: Query<
        (
            &mut ChunkLayer,
            &WorldBorderCenter,
            &WorldBorderLerp,
            &WorldBorderPortalTpBoundary,
            &WorldBorderWarnTime,
            &WorldBorderWarnBlocks,
        ),
        Changed<WorldBorderPortalTpBoundary>,
    >,
    server: Res<Server>,
) {
    for (mut layer, center, lerp, portal_tp_boundary, warn_time, warn_blocks) in &mut wbs {
        let millis = lerp.remaining_ticks as i64 * 1000 / server.tick_rate().get() as i64;

        layer.write_packet(&WorldBorderInitializeS2c {
            x: center.x,
            z: center.z,
            old_diameter: lerp.current_diameter,
            new_diameter: lerp.target_diameter,
            duration_millis: millis.into(),
            portal_teleport_boundary: portal_tp_boundary.0.into(),
            warning_blocks: warn_blocks.0.into(),
            warning_time: warn_time.0.into(),
        });
    }
}
