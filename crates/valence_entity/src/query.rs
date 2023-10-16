use std::mem;

use bevy_ecs::prelude::DetectChanges;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::world::Ref;
use valence_math::DVec3;
use valence_protocol::encode::WritePacket;
use valence_protocol::packets::play::{
    EntityAnimationS2c, EntityAttributesS2c, EntityPositionS2c, EntitySetHeadYawS2c,
    EntitySpawnS2c, EntityStatusS2c, EntityTrackerUpdateS2c, EntityVelocityUpdateS2c,
    ExperienceOrbSpawnS2c, MoveRelativeS2c, PlayerSpawnS2c, RotateAndMoveRelativeS2c, RotateS2c,
};
use valence_protocol::var_int::VarInt;
use valence_protocol::ByteAngle;
use valence_server_common::UniqueId;

use crate::attributes::TrackedEntityAttributes;
use crate::tracked_data::TrackedData;
use crate::{
    EntityAnimations, EntityId, EntityKind, EntityLayerId, EntityStatuses, HeadYaw, Look,
    ObjectData, OldEntityLayerId, OldPosition, OnGround, Position, Velocity,
};

#[derive(WorldQuery)]
pub struct EntityInitQuery {
    pub entity_id: &'static EntityId,
    pub uuid: &'static UniqueId,
    pub kind: &'static EntityKind,
    pub look: &'static Look,
    pub head_yaw: &'static HeadYaw,
    pub on_ground: &'static OnGround,
    pub object_data: &'static ObjectData,
    pub velocity: &'static Velocity,
    pub tracked_data: &'static TrackedData,
}

impl EntityInitQueryItem<'_> {
    /// Writes the appropriate packets to initialize an entity. This will spawn
    /// the entity and initialize tracked data. `pos` is the initial position of
    /// the entity.
    pub fn write_init_packets(&self, pos: DVec3, mut writer: impl WritePacket) {
        match *self.kind {
            EntityKind::MARKER => {}
            EntityKind::EXPERIENCE_ORB => {
                writer.write_packet(&ExperienceOrbSpawnS2c {
                    entity_id: self.entity_id.get().into(),
                    position: pos,
                    count: self.object_data.0 as i16,
                });
            }
            EntityKind::PLAYER => {
                writer.write_packet(&PlayerSpawnS2c {
                    entity_id: self.entity_id.get().into(),
                    player_uuid: self.uuid.0,
                    position: pos,
                    yaw: ByteAngle::from_degrees(self.look.yaw),
                    pitch: ByteAngle::from_degrees(self.look.pitch),
                });

                // Player spawn packet doesn't include head yaw for some reason.
                writer.write_packet(&EntitySetHeadYawS2c {
                    entity_id: self.entity_id.get().into(),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
                });
            }
            _ => writer.write_packet(&EntitySpawnS2c {
                entity_id: self.entity_id.get().into(),
                object_uuid: self.uuid.0,
                kind: self.kind.get().into(),
                position: pos,
                pitch: ByteAngle::from_degrees(self.look.pitch),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
                data: self.object_data.0.into(),
                velocity: self.velocity.to_packet_units(),
            }),
        }

        if let Some(init_data) = self.tracked_data.init_data() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id: self.entity_id.get().into(),
                tracked_values: init_data.into(),
            });
        }
    }
}

#[derive(WorldQuery)]
pub struct UpdateEntityQuery {
    pub id: &'static EntityId,
    pub pos: &'static Position,
    pub old_pos: &'static OldPosition,
    pub loc: &'static EntityLayerId,
    pub old_loc: &'static OldEntityLayerId,
    pub look: Ref<'static, Look>,
    pub head_yaw: Ref<'static, HeadYaw>,
    pub on_ground: &'static OnGround,
    pub velocity: Ref<'static, Velocity>,
    pub tracked_data: &'static TrackedData,
    pub statuses: &'static EntityStatuses,
    pub animations: &'static EntityAnimations,
    // Option because not all entities have attributes, only LivingEntity.
    pub tracked_attributes: Option<&'static TrackedEntityAttributes>,
}

impl UpdateEntityQueryItem<'_> {
    pub fn write_update_packets(&self, mut writer: impl WritePacket) {
        // TODO: @RJ I saw you're using UpdateEntityPosition and UpdateEntityRotation sometimes. These two packets are actually broken on the client and will erase previous position/rotation https://bugs.mojang.com/browse/MC-255263 -Moulberry

        let entity_id = VarInt(self.id.get());

        let position_delta = self.pos.0 - self.old_pos.get();
        let needs_teleport = position_delta.abs().max_element() >= 8.0;
        let changed_position = self.pos.0 != self.old_pos.get();

        if changed_position && !needs_teleport && self.look.is_changed() {
            writer.write_packet(&RotateAndMoveRelativeS2c {
                entity_id,
                delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                pitch: ByteAngle::from_degrees(self.look.pitch),
                on_ground: self.on_ground.0,
            });
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&MoveRelativeS2c {
                    entity_id,
                    delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                    on_ground: self.on_ground.0,
                });
            }

            if self.look.is_changed() {
                writer.write_packet(&RotateS2c {
                    entity_id,
                    yaw: ByteAngle::from_degrees(self.look.yaw),
                    pitch: ByteAngle::from_degrees(self.look.pitch),
                    on_ground: self.on_ground.0,
                });
            }
        }

        if needs_teleport {
            writer.write_packet(&EntityPositionS2c {
                entity_id,
                position: self.pos.0,
                yaw: ByteAngle::from_degrees(self.look.yaw),
                pitch: ByteAngle::from_degrees(self.look.pitch),
                on_ground: self.on_ground.0,
            });
        }

        if self.velocity.is_changed() {
            writer.write_packet(&EntityVelocityUpdateS2c {
                entity_id,
                velocity: self.velocity.to_packet_units(),
            });
        }

        if self.head_yaw.is_changed() {
            writer.write_packet(&EntitySetHeadYawS2c {
                entity_id,
                head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
            });
        }

        if let Some(update_data) = self.tracked_data.update_data() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id,
                tracked_values: update_data.into(),
            });
        }

        if self.statuses.0 != 0 {
            for i in 0..mem::size_of_val(self.statuses) {
                if (self.statuses.0 >> i) & 1 == 1 {
                    writer.write_packet(&EntityStatusS2c {
                        entity_id: entity_id.0,
                        entity_status: i as u8,
                    });
                }
            }
        }

        if self.animations.0 != 0 {
            for i in 0..mem::size_of_val(self.animations) {
                if (self.animations.0 >> i) & 1 == 1 {
                    writer.write_packet(&EntityAnimationS2c {
                        entity_id,
                        animation: i as u8,
                    });
                }
            }
        }

        if let Some(attributes) = self.tracked_attributes {
            let properties = attributes.get_properties();

            if !properties.is_empty() {
                writer.write_packet(&EntityAttributesS2c {
                    entity_id,
                    properties,
                });
            }
        }
    }
}
