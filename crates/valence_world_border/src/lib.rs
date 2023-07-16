//! # World border
//! This module contains Components and Systems needed to handle world border.
//!
//! The world border is the current edge of a Minecraft dimension. It appears as
//! a series of animated, diagonal, narrow stripes. For more information, refer to the [wiki](https://minecraft.fandom.com/wiki/World_border)
//!
//! ## Enable world border per instance
//! By default, world border is not enabled. It can be enabled by inserting the
//! [`WorldBorderBundle`] bundle into a [`Instance`].
//! Use [`WorldBorderBundle::default()`] to use Minecraft Vanilla border default
//! ```
//! commands
//!     .entity(instance_entity)
//!     .insert(WorldBorderBundle::new([0.0, 0.0], 10.0));
//! ```
//!
//!
//! ## Modify world border diameter
//! World border diameter can be changed using [`SetWorldBorderSizeEvent`].
//! Setting duration to 0 will move the border to `new_diameter` immediately,
//! otherwise, it will interpolate to `new_diameter` over `duration` time.
//! ```
//! fn change_diameter(
//!     event_writer: EventWriter<SetWorldBorderSizeEvent>,
//!     diameter: f64,
//!     duration: Duration,
//! ) {
//!     event_writer.send(SetWorldBorderSizeEvent {
//!         instance: entity,
//!         new_diameter: diameter,
//!         duration,
//!     })
//! }
//! ```
//!
//! You can also modify the [`MovingWorldBorder`] if you want more control. But
//! it is not recommended.
//!
//! ## Querying world border diameter
//! World border diameter can be read by querying
//! [`WorldBorderDiameter::get()`]. Note: If you want to modify the
//! diameter size, do not modify the value directly! Use
//! [`SetWorldBorderSizeEvent`] instead.
//!
//! ## Access other world border properties.
//! Access to the rest of the world border properties is fairly straightforward
//! by querying their respective component. [`WorldBorderBundle`] contains
//! references for all properties of the world border and their respective
//! component
#![allow(clippy::type_complexity)]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

// TODO: fix.

/*
pub mod packet;

use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use glam::DVec2;
use packet::*;
use valence_client::{Client, FlushPacketsSet};
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::var_long::VarLong;
use valence_entity::EntityLayerId;
use valence_layer::UpdateLayersPreClientSet;
use valence_registry::*;

// https://minecraft.fandom.com/wiki/World_border
pub const DEFAULT_PORTAL_LIMIT: i32 = 29999984;
pub const DEFAULT_DIAMETER: f64 = (DEFAULT_PORTAL_LIMIT * 2) as f64;
pub const DEFAULT_WARN_TIME: i32 = 15;
pub const DEFAULT_WARN_BLOCKS: i32 = 5;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWorldBorderPerInstanceSet;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWorldBorderPerClientSet;

pub struct WorldBorderPlugin;

impl Plugin for WorldBorderPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                UpdateWorldBorderPerInstanceSet.before(UpdateLayersPreClientSet),
                UpdateWorldBorderPerClientSet.before(FlushPacketsSet),
            ),
        )
        .add_event::<SetWorldBorderSizeEvent>()
        .add_systems(
            PostUpdate,
            (
                wb_size_change.before(diameter_change),
                diameter_change,
                lerp_transition,
                center_change,
                warn_time_change,
                warn_blocks_change,
                portal_teleport_boundary_change,
            )
                .in_set(UpdateWorldBorderPerInstanceSet),
        )
        .add_systems(
            PostUpdate,
            border_for_player.in_set(UpdateWorldBorderPerClientSet),
        );
    }
}

/// A bundle contains necessary component to enable world border.
/// This struct implements [`Default`] trait that returns a bundle using
/// Minecraft Vanilla defaults.
#[derive(Bundle)]
pub struct WorldBorderBundle {
    pub center: WorldBorderCenter,
    pub diameter: WorldBorderDiameter,
    pub portal_teleport_boundary: WorldBorderPortalTpBoundary,
    pub warning_time: WorldBorderWarnTime,
    pub warning_blocks: WorldBorderWarnBlocks,
    pub moving: MovingWorldBorder,
}

impl WorldBorderBundle {
    /// Create a new world border with specified center and diameter
    pub fn new(center: impl Into<DVec2>, diameter: f64) -> Self {
        Self {
            center: WorldBorderCenter(center.into()),
            diameter: WorldBorderDiameter(diameter),
            portal_teleport_boundary: WorldBorderPortalTpBoundary(DEFAULT_PORTAL_LIMIT),
            warning_time: WorldBorderWarnTime(DEFAULT_WARN_TIME),
            warning_blocks: WorldBorderWarnBlocks(DEFAULT_WARN_BLOCKS),
            moving: MovingWorldBorder {
                old_diameter: diameter,
                new_diameter: diameter,
                duration: 0,
                timestamp: Instant::now(),
            },
        }
    }
}

impl Default for WorldBorderBundle {
    fn default() -> Self {
        Self::new([0.0, 0.0], DEFAULT_DIAMETER)
    }
}

#[derive(Component)]
pub struct WorldBorderCenter(pub DVec2);

#[derive(Component)]
pub struct WorldBorderWarnTime(pub i32);

#[derive(Component)]
pub struct WorldBorderWarnBlocks(pub i32);

#[derive(Component)]
pub struct WorldBorderPortalTpBoundary(pub i32);

/// The world border diameter can be read by calling
/// [`WorldBorderDiameter::get()`]. If you want to modify the diameter
/// size, do not modify the value directly! Use [`SetWorldBorderSizeEvent`]
/// instead.
#[derive(Component)]
pub struct WorldBorderDiameter(f64);

impl WorldBorderDiameter {
    pub fn get(&self) -> f64 {
        self.0
    }
}

/// This component represents the `Set Border Lerp Size` packet with timestamp.
/// It is used for actually lerping the world border diameter.
/// If you need to set the diameter, it is much better to use the
/// [`SetWorldBorderSizeEvent`] event
#[derive(Component)]
pub struct MovingWorldBorder {
    pub old_diameter: f64,
    pub new_diameter: f64,
    /// equivalent to `speed` on wiki.vg
    pub duration: i64,
    pub timestamp: Instant,
}

impl MovingWorldBorder {
    pub fn current_diameter(&self) -> f64 {
        if self.duration == 0 {
            self.new_diameter
        } else {
            let t = self.current_duration() as f64 / self.duration as f64;
            lerp(self.new_diameter, self.old_diameter, t)
        }
    }

    pub fn current_duration(&self) -> i64 {
        let speed = self.duration - self.timestamp.elapsed().as_millis() as i64;
        speed.max(0)
    }
}

/// An event for controlling world border diameter.
/// Setting duration to 0 will move the border to `new_diameter` immediately,
/// otherwise it will interpolate to `new_diameter` over `duration` time.
///
/// ```
/// fn change_diameter(
///     event_writer: EventWriter<SetWorldBorderSizeEvent>,
///     diameter: f64,
///     duration: Duration,
/// ) {
///     event_writer.send(SetWorldBorderSizeEvent {
///         entity_layer: entity,
///         new_diameter: diameter,
///         duration,
///     });
/// }
/// ```
#[derive(Event, Clone, Debug)]
pub struct SetWorldBorderSizeEvent {
    /// The [`EntityLayer`] to change border size. Note that this entity layer must contain
    /// the [`WorldBorderBundle`] bundle.
    pub entity_layer: Entity,
    /// The new diameter of the world border
    pub new_diameter: f64,
    /// How long the border takes to reach it new_diameter in millisecond. Set
    /// to 0 to move immediately.
    pub duration: Duration,
}

fn wb_size_change(
    mut events: EventReader<SetWorldBorderSizeEvent>,
    mut instances: Query<(&WorldBorderDiameter, Option<&mut MovingWorldBorder>)>,
) {
    for SetWorldBorderSizeEvent {
        entity_layer: instance,
        new_diameter,
        duration,
    } in events.iter()
    {
        let Ok((diameter, mwb_opt)) = instances.get_mut(*instance) else {
            continue;
        };

        if let Some(mut mvb) = mwb_opt {
            mvb.new_diameter = *new_diameter;
            mvb.old_diameter = diameter.get();
            mvb.duration = duration.as_millis() as i64;
            mvb.timestamp = Instant::now();
        }
    }
}

fn border_for_player(
    mut clients: Query<(&mut Client, &EntityLayerId), Changed<EntityLayerId>>,
    wbs: Query<
        (
            &WorldBorderCenter,
            &WorldBorderWarnTime,
            &WorldBorderWarnBlocks,
            &WorldBorderDiameter,
            &WorldBorderPortalTpBoundary,
            Option<&MovingWorldBorder>,
        ),
        With<Instance>,
    >,
) {
    for (mut client, location) in clients.iter_mut() {
        if let Ok((c, wt, wb, diameter, ptb, wbl)) = wbs.get(location.0) {
            let (new_diameter, speed) = if let Some(lerping) = wbl {
                (lerping.new_diameter, lerping.current_duration())
            } else {
                (diameter.0, 0)
            };

            client.write_packet(&WorldBorderInitializeS2c {
                x: c.0.x,
                z: c.0.y,
                old_diameter: diameter.0,
                new_diameter,
                portal_teleport_boundary: VarInt(ptb.0),
                speed: VarLong(speed),
                warning_blocks: VarInt(wb.0),
                warning_time: VarInt(wt.0),
            });
        }
    }
}

fn diameter_change(
    mut wbs: Query<(&mut Instance, &MovingWorldBorder), Changed<MovingWorldBorder>>,
) {
    for (mut ins, lerping) in wbs.iter_mut() {
        if lerping.duration == 0 {
            ins.write_packet(&WorldBorderSizeChangedS2c {
                diameter: lerping.new_diameter,
            })
        } else {
            ins.write_packet(&WorldBorderInterpolateSizeS2c {
                old_diameter: lerping.current_diameter(),
                new_diameter: lerping.new_diameter,
                speed: VarLong(lerping.current_duration()),
            });
        }
    }
}

fn lerp_transition(mut wbs: Query<(&mut WorldBorderDiameter, &MovingWorldBorder)>) {
    for (mut diameter, moving_wb) in wbs.iter_mut() {
        if diameter.0 != moving_wb.new_diameter {
            diameter.0 = moving_wb.current_diameter();
        }
    }
}

fn center_change(mut wbs: Query<(&mut Instance, &WorldBorderCenter), Changed<WorldBorderCenter>>) {
    for (mut ins, center) in wbs.iter_mut() {
        ins.write_packet(&WorldBorderCenterChangedS2c {
            x_pos: center.0.x,
            z_pos: center.0.y,
        })
    }
}

fn warn_time_change(
    mut wb_query: Query<(&mut Instance, &WorldBorderWarnTime), Changed<WorldBorderWarnTime>>,
) {
    for (mut ins, wt) in wb_query.iter_mut() {
        ins.write_packet(&WorldBorderWarningTimeChangedS2c {
            warning_time: VarInt(wt.0),
        })
    }
}

fn warn_blocks_change(
    mut wb_query: Query<(&mut Instance, &WorldBorderWarnBlocks), Changed<WorldBorderWarnBlocks>>,
) {
    for (mut ins, wb) in wb_query.iter_mut() {
        ins.write_packet(&WorldBorderWarningBlocksChangedS2c {
            warning_blocks: VarInt(wb.0),
        })
    }
}

fn portal_teleport_boundary_change(
    mut wbs: Query<
        (
            &mut Instance,
            &WorldBorderCenter,
            &WorldBorderWarnTime,
            &WorldBorderWarnBlocks,
            &WorldBorderDiameter,
            &WorldBorderPortalTpBoundary,
            Option<&MovingWorldBorder>,
        ),
        Changed<WorldBorderPortalTpBoundary>,
    >,
) {
    for (mut ins, c, wt, wb, diameter, ptb, wbl) in wbs.iter_mut() {
        let (new_diameter, speed) = if let Some(lerping) = wbl {
            (lerping.new_diameter, lerping.current_duration())
        } else {
            (diameter.0, 0)
        };

        ins.write_packet(&WorldBorderInitializeS2c {
            x: c.0.x,
            z: c.0.y,
            old_diameter: diameter.0,
            new_diameter,
            portal_teleport_boundary: VarInt(ptb.0),
            speed: VarLong(speed),
            warning_blocks: VarInt(wb.0),
            warning_time: VarInt(wt.0),
        });
    }
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}
*/
