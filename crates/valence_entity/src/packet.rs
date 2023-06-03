use std::borrow::Cow;
use std::io::Write;

use bitfield_struct::bitfield;
use glam::DVec3;
use uuid::Uuid;
use valence_core::ident::Ident;
use valence_core::item::ItemStack;
use valence_core::protocol::byte_angle::ByteAngle;
use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_nbt::Compound;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITIES_DESTROY_S2C)]
pub struct EntitiesDestroyS2c<'a> {
    pub entity_ids: Cow<'a, [VarInt]>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ANIMATION_S2C)]
pub struct EntityAnimationS2c {
    pub entity_id: VarInt,
    pub animation: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ATTACH_S2C)]
pub struct EntityAttachS2c {
    pub attached_entity_id: i32,
    pub holding_entity_id: i32,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ATTRIBUTES_S2C)]
pub struct EntityAttributesS2c<'a> {
    pub entity_id: VarInt,
    pub properties: Vec<AttributeProperty<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeProperty<'a> {
    pub key: Ident<Cow<'a, str>>,
    pub value: f64,
    pub modifiers: Vec<AttributeModifier>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeModifier {
    pub uuid: Uuid,
    pub amount: f64,
    pub operation: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_DAMAGE_S2C)]
pub struct EntityDamageS2c {
    /// The ID of the entity taking damage
    pub entity_id: VarInt,
    /// The ID of the type of damage taken
    pub source_type_id: VarInt,
    /// The ID + 1 of the entity responsible for the damage, if present. If not
    /// present, the value is 0
    pub source_cause_id: VarInt,
    /// The ID + 1 of the entity that directly dealt the damage, if present. If
    /// not present, the value is 0. If this field is present:
    /// * and damage was dealt indirectly, such as by the use of a projectile,
    ///   this field will contain the ID of such projectile;
    /// * and damage was dealt dirctly, such as by manually attacking, this
    ///   field will contain the same value as Source Cause ID.
    pub source_direct_id: VarInt,
    /// The Notchian server sends the Source Position when the damage was dealt
    /// by the /damage command and a position was specified
    pub source_pos: Option<DVec3>,
}

#[derive(Clone, PartialEq, Debug, Packet)]
#[packet(id = packet_id::ENTITY_EQUIPMENT_UPDATE_S2C)]
pub struct EntityEquipmentUpdateS2c {
    pub entity_id: VarInt,
    pub equipment: Vec<EquipmentEntry>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct EquipmentEntry {
    pub slot: i8,
    pub item: Option<ItemStack>,
}

impl Encode for EntityEquipmentUpdateS2c {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.entity_id.encode(&mut w)?;

        for i in 0..self.equipment.len() {
            let slot = self.equipment[i].slot;
            if i != self.equipment.len() - 1 {
                (slot | -128).encode(&mut w)?;
            } else {
                slot.encode(&mut w)?;
            }
            self.equipment[i].item.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for EntityEquipmentUpdateS2c {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let entity_id = VarInt::decode(r)?;

        let mut equipment = vec![];

        loop {
            let slot = i8::decode(r)?;
            let item = Option::<ItemStack>::decode(r)?;
            equipment.push(EquipmentEntry {
                slot: slot & 127,
                item,
            });
            if slot & -128 == 0 {
                break;
            }
        }

        Ok(Self {
            entity_id,
            equipment,
        })
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::MOVE_RELATIVE)]
pub struct MoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ROTATE_AND_MOVE_RELATIVE)]
pub struct RotateAndMoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ROTATE)]
pub struct RotateS2c {
    pub entity_id: VarInt,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_PASSENGERS_SET_S2C)]
pub struct EntityPassengersSetS2c {
    /// Vehicle's entity id
    pub entity_id: VarInt,
    pub passengers: Vec<VarInt>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_POSITION_S2C)]
pub struct EntityPositionS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_SET_HEAD_YAW_S2C)]
pub struct EntitySetHeadYawS2c {
    pub entity_id: VarInt,
    pub head_yaw: ByteAngle,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_SPAWN_S2C)]
pub struct EntitySpawnS2c {
    pub entity_id: VarInt,
    pub object_uuid: Uuid,
    pub kind: VarInt,
    pub position: DVec3,
    pub pitch: ByteAngle,
    pub yaw: ByteAngle,
    pub head_yaw: ByteAngle,
    pub data: VarInt,
    pub velocity: [i16; 3],
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_STATUS_EFFECT_S2C)]
pub struct EntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
    pub amplifier: u8,
    pub duration: VarInt,
    pub flags: Flags,
    pub factor_codec: Option<Compound>,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub is_ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[bits(5)]
    _pad: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_STATUS_S2C)]
pub struct EntityStatusS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_TRACKER_UPDATE_S2C)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub metadata: RawBytes<'a>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_VELOCITY_UPDATE_S2C)]
pub struct EntityVelocityUpdateS2c {
    pub entity_id: VarInt,
    pub velocity: [i16; 3],
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPERIENCE_ORB_SPAWN_S2C)]
pub struct ExperienceOrbSpawnS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    pub count: i16,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REMOVE_ENTITY_STATUS_EFFECT_S2C)]
pub struct RemoveEntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ITEM_PICKUP_ANIMATION_S2C)]
pub struct ItemPickupAnimationS2c {
    pub collected_entity_id: VarInt,
    pub collector_entity_id: VarInt,
    pub pickup_item_count: VarInt,
}

/// Instructs a client to face an entity.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOOK_AT_S2C)]
pub struct LookAtS2c {
    pub feet_or_eyes: FeetOrEyes,
    pub target_position: DVec3,
    pub entity_to_face: Option<LookAtEntity>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum FeetOrEyes {
    Feet,
    Eyes,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct LookAtEntity {
    pub entity_id: VarInt,
    pub feet_or_eyes: FeetOrEyes,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SET_CAMERA_ENTITY_S2C)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
