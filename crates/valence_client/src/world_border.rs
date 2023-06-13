// Things to update:
// - use system clock as reference time instead of server tick [done]
// - Update portal tp boundary wont send packet to update because there is no
//   packet to specifically to update it. Maybe use InitializeWorldBorderS2c?

use glam::DVec2;
use valence_core::protocol::var_long::VarLong;
use valence_entity::Location;
use valence_instance::packet::*;
use valence_registry::{Component, Query};

use crate::*;

// https://minecraft.fandom.com/wiki/World_border
pub const DEFAULT_PORTAL_LIMIT: i32 = 29999984;
pub const DEFAULT_DIAMETER: f64 = (DEFAULT_PORTAL_LIMIT * 2) as f64;
pub const DEFAULT_WARN_TIME: i32 = 15;
pub const DEFAULT_WARN_BLOCKS: i32 = 5;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWorldBorderPerInstanceSet;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateWorldBorderPerClientSet;

pub(crate) fn build(app: &mut App) {
    app.configure_set(
        UpdateWorldBorderPerInstanceSet
            .in_base_set(CoreSet::PostUpdate)
            .before(WriteUpdatePacketsToInstancesSet),
    )
    .configure_set(
        UpdateWorldBorderPerClientSet
            .in_base_set(CoreSet::PostUpdate)
            .before(FlushPacketsSet),
    )
    .add_event::<SetWorldBorderSizeEvent>()
    .add_systems(
        (
            handle_wb_size_change.before(handle_diameter_change),
            handle_diameter_change,
            handle_lerp_transition,
            handle_center_change,
            handle_warn_time_change,
            handle_warn_blocks_change,
            handle_portal_teleport_bounary_change,
        )
            .in_set(UpdateWorldBorderPerInstanceSet),
    )
    .add_system(handle_border_for_player.in_set(UpdateWorldBorderPerClientSet));
}

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
                speed: 0,
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

#[derive(Component)]
pub struct WorldBorderDiameter(f64);

impl WorldBorderDiameter {
    pub fn diameter(&self) -> f64 {
        self.0
    }
}

#[derive(Component)]
pub struct MovingWorldBorder {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: i64,
    pub timestamp: Instant,
}

impl MovingWorldBorder {
    pub fn current_diameter(&self) -> f64 {
        let t = self.current_speed() as f64 / self.speed as f64;
        lerp(self.new_diameter, self.old_diameter, t)
    }

    pub fn current_speed(&self) -> i64 {
        let speed = self.speed - self.timestamp.elapsed().as_millis() as i64;
        speed.max(0)
    }
}

pub struct SetWorldBorderSizeEvent {
    pub instance: Entity,
    pub new_diameter: f64,
    pub speed: i64,
}

fn handle_wb_size_change(
    mut events: EventReader<SetWorldBorderSizeEvent>,
    mut instances: Query<(Entity, &WorldBorderDiameter, Option<&mut MovingWorldBorder>)>,
    mut commands: Commands,
) {
    for SetWorldBorderSizeEvent {
        instance,
        new_diameter,
        speed,
    } in events.iter()
    {
        let Ok((entity, diameter, mwb_opt)) = instances.get_mut(instance.clone()) else {
            continue;
        };

        if let Some(mut mvb) = mwb_opt {
            mvb.new_diameter = *new_diameter;
            mvb.old_diameter = diameter.diameter();
            mvb.speed = *speed;
            mvb.timestamp = Instant::now();
        } else {
            // This might be delayed by 1 tick
            commands.entity(entity).insert(MovingWorldBorder {
                new_diameter: *new_diameter,
                old_diameter: diameter.diameter(),
                speed: *speed,
                timestamp: Instant::now(),
            });
        }
    }
}

fn handle_border_for_player(
    mut clients: Query<(&mut Client, &Location), Changed<Location>>,
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
                (lerping.new_diameter, lerping.current_speed())
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

fn handle_diameter_change(
    mut wbs: Query<(&mut Instance, &MovingWorldBorder), Changed<MovingWorldBorder>>,
) {
    for (mut ins, lerping) in wbs.iter_mut() {
        if lerping.speed == 0 {
            ins.write_packet(&WorldBorderSizeChangedS2c {
                diameter: lerping.new_diameter,
            })
        } else {
            ins.write_packet(&WorldBorderInterpolateSizeS2c {
                old_diameter: lerping.current_diameter(),
                new_diameter: lerping.new_diameter,
                speed: VarLong(lerping.current_speed()),
            });
        }
    }
}

fn handle_lerp_transition(mut wbs: Query<(&mut WorldBorderDiameter, &MovingWorldBorder)>) {
    for (mut diameter, moving_wb) in wbs.iter_mut() {
        if diameter.0 != moving_wb.new_diameter {
            diameter.0 = moving_wb.current_diameter();
        }
    }
}

fn handle_center_change(
    mut wbs: Query<(&mut Instance, &WorldBorderCenter), Changed<WorldBorderCenter>>,
) {
    for (mut ins, center) in wbs.iter_mut() {
        ins.write_packet(&WorldBorderCenterChangedS2c {
            x_pos: center.0.x,
            z_pos: center.0.y,
        })
    }
}

fn handle_warn_time_change(
    mut wb_query: Query<(&mut Instance, &WorldBorderWarnTime), Changed<WorldBorderWarnTime>>,
) {
    for (mut ins, wt) in wb_query.iter_mut() {
        ins.write_packet(&WorldBorderWarningTimeChangedS2c {
            warning_time: VarInt(wt.0),
        })
    }
}

fn handle_warn_blocks_change(
    mut wb_query: Query<(&mut Instance, &WorldBorderWarnBlocks), Changed<WorldBorderWarnBlocks>>,
) {
    for (mut ins, wb) in wb_query.iter_mut() {
        ins.write_packet(&WorldBorderWarningBlocksChangedS2c {
            warning_blocks: VarInt(wb.0),
        })
    }
}

fn handle_portal_teleport_bounary_change(
    mut wbs: Query<(
        &mut Instance,
        &WorldBorderCenter,
        &WorldBorderWarnTime,
        &WorldBorderWarnBlocks,
        &WorldBorderDiameter,
        &WorldBorderPortalTpBoundary,
        Option<&MovingWorldBorder>,
    )>,
) {
    for (mut ins, c, wt, wb, diameter, ptb, wbl) in wbs.iter_mut() {
        let (new_diameter, speed) = if let Some(lerping) = wbl {
            (lerping.new_diameter, lerping.current_speed())
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
