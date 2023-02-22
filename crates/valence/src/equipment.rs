use bevy_ecs::prelude::*;
use bevy_ecs::{query::Changed, system::Query};
use valence_protocol::packets::s2c::set_equipment::SetEquipment;
use valence_protocol::VarInt;
use valence_protocol::{packets::s2c::set_equipment::EquipmentEntry, ItemStack};

use crate::prelude::*;
use crate::view::ChunkPos;

/// ECS component to be added for entities with equipment.
///
/// Equipment updates managed by `update_equipment`.
#[derive(Component, Default, PartialEq, Debug)]
pub struct Equipment {
    equipment: Vec<EquipmentEntry>,
    /// Bit set with the modified equipment slots
    modified_slots: u8,
}

#[derive(Copy, Clone)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Boots,
    Leggings,
    Chestplate,
    Helmet,
}

impl Equipment {
    pub fn new() -> Equipment {
        Equipment::default()
    }

    /// Set an equipment slot with an item stack
    pub fn set(&mut self, item: ItemStack, slot: EquipmentSlot) {
        if let Some(equip) = self.get_mut(slot) {
            equip.item = Some(item);
        } else {
            self.equipment.push(EquipmentEntry {
                item: Some(item),
                slot: slot.into(),
            });
        }

        self.set_modified_slot(slot);
    }

    /// Remove all equipment
    pub fn clear(&mut self) {
        for equip in self.equipment.iter() {
            self.modified_slots |= 1 << equip.slot as u8;
        }

        self.equipment.clear();
    }

    /// Remove an equipment from a slot and return it if present
    pub fn remove(&mut self, slot: EquipmentSlot) -> Option<EquipmentEntry> {
        let slot_id: i8 = slot.into();

        if let Some(idx) = self
            .equipment
            .iter()
            .position(|equip| equip.slot == slot_id)
        {
            self.set_modified_slot(slot);
            Some(self.equipment.remove(idx))
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, slot: EquipmentSlot) -> Option<&mut EquipmentEntry> {
        let slot: i8 = slot.into();
        self.equipment.iter_mut().find(|equip| equip.slot == slot)
    }

    pub fn get(&self, slot: EquipmentSlot) -> Option<&EquipmentEntry> {
        let slot: i8 = slot.into();
        self.equipment.iter().find(|equip| equip.slot == slot)
    }

    pub fn equipment(&self) -> &Vec<EquipmentEntry> {
        &self.equipment
    }

    pub fn is_empty(&self) -> bool {
        self.equipment.is_empty()
    }

    fn has_modified_slots(&self) -> bool {
        self.modified_slots != 0
    }

    fn iter_modified_equipment(&self) -> impl Iterator<Item = EquipmentEntry> + '_ {
        self.iter_modified_slots().map(|slot| {
            self.get(slot).cloned().unwrap_or_else(|| EquipmentEntry {
                slot: slot.into(),
                item: None,
            })
        })
    }

    fn iter_modified_slots(&self) -> impl Iterator<Item = EquipmentSlot> {
        let modified_slots = self.modified_slots;

        (0..=5).filter_map(move |slot: i8| {
            if modified_slots & (1 << slot) != 0 {
                Some(EquipmentSlot::try_from(slot).unwrap())
            } else {
                None
            }
        })
    }

    fn set_modified_slot(&mut self, slot: EquipmentSlot) {
        let shifts: i8 = slot.into();
        self.modified_slots |= 1 << (shifts as u8);
    }

    fn clear_modified_slot(&mut self) {
        self.modified_slots = 0;
    }
}

impl TryFrom<i8> for EquipmentSlot {
    type Error = &'static str;

    /// Convert from `id` according to <https://wiki.vg/Protocol#Set_Equipment>
    fn try_from(id: i8) -> Result<Self, Self::Error> {
        let slot = match id {
            0 => EquipmentSlot::MainHand,
            1 => EquipmentSlot::OffHand,
            2 => EquipmentSlot::Boots,
            3 => EquipmentSlot::Leggings,
            4 => EquipmentSlot::Chestplate,
            5 => EquipmentSlot::Helmet,
            _ => return Err("Invalid value"),
        };

        Ok(slot)
    }
}

impl From<EquipmentSlot> for i8 {
    /// Convert to `id` according to <https://wiki.vg/Protocol#Set_Equipment>
    fn from(slot: EquipmentSlot) -> Self {
        match slot {
            EquipmentSlot::MainHand => 0,
            EquipmentSlot::OffHand => 1,
            EquipmentSlot::Boots => 2,
            EquipmentSlot::Leggings => 3,
            EquipmentSlot::Chestplate => 4,
            EquipmentSlot::Helmet => 5,
        }
    }
}

/// When a [Equipment] component is changed, send [SetEquipment] packet to all clients
/// that have the updated entity in their view distance.
///
/// NOTE: [SetEquipment] packet only have cosmetic effect, which means it does not affect armor resistance or damage.
pub(crate) fn update_equipment(
    mut equiped_entities: Query<(&McEntity, &mut Equipment), Changed<Equipment>>,
    mut instances: Query<&mut Instance>,
) {
    for (equiped_mc_entity, mut equips) in &mut equiped_entities {
        if !equips.has_modified_slots() {
            continue;
        }

        let instance = equiped_mc_entity.instance();
        let chunk_pos = ChunkPos::from_dvec3(equiped_mc_entity.position());

        if let Ok(mut instance) = instances.get_mut(instance) {
            instance.write_packet_at(
                &SetEquipment {
                    entity_id: VarInt(equiped_mc_entity.protocol_id()),
                    equipment: equips.iter_modified_equipment().collect(),
                },
                chunk_pos,
            )
        }

        equips.clear_modified_slot();
    }
}

#[cfg(test)]
mod test {
    use valence_protocol::packets::S2cPlayPacket;

    use crate::unit_test::util::scenario_single_client;

    use super::*;

    #[test]
    fn test_set_boots_and_clear() {
        let mut equipment = Equipment::default();
        assert_eq!(
            equipment,
            Equipment {
                equipment: vec![],
                modified_slots: 0
            }
        );

        let item = ItemStack::new(ItemKind::GreenWool, 1, None);
        let slot = EquipmentSlot::Boots;
        equipment.set(item.clone(), slot);

        assert_eq!(
            equipment,
            Equipment {
                equipment: vec![EquipmentEntry {
                    slot: slot.into(),
                    item: Some(item)
                }],
                modified_slots: 0b100
            }
        );

        equipment.clear_modified_slot();
        equipment.clear();
        assert_eq!(
            equipment,
            Equipment {
                equipment: vec![],
                modified_slots: 0b100
            }
        );
        assert_eq!(
            equipment
                .iter_modified_equipment()
                .collect::<Vec<EquipmentEntry>>(),
            vec![EquipmentEntry {
                slot: slot.into(),
                item: None
            }]
        );
    }

    #[test]
    fn test_set_main_hand_and_remove_it() {
        let mut equipment = Equipment::default();

        let item = ItemStack::new(ItemKind::DiamondSword, 1, None);
        let slot = EquipmentSlot::MainHand;
        equipment.set(item.clone(), slot);

        assert_eq!(
            equipment.remove(EquipmentSlot::MainHand),
            Some(EquipmentEntry {
                slot: slot.into(),
                item: Some(item)
            })
        );
        assert_eq!(equipment.remove(EquipmentSlot::Helmet), None);
        assert_eq!(
            equipment,
            Equipment {
                equipment: vec![],
                modified_slots: 0b1
            }
        );
    }

    #[test]
    fn test_set_equipment_sent_packets() -> anyhow::Result<()> {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        // Setup server
        let mut instance = app
            .world
            .resource::<Server>()
            .new_instance(DimensionId::default());
        instance.insert_chunk([0, 0], Default::default());
        let instance = app.world.spawn(instance);
        let instance_entity = instance.id();

        // Setup client
        let mut client = app.world.get_mut::<Client>(client_ent).unwrap();
        let uuid = client.uuid();
        client.set_position([0.0, 0.0, 0.0]);
        client.set_instance(instance_entity);
        let mut client_ent_mut = app
            .world
            .get_entity_mut(client_ent)
            .expect("should have client component");
        client_ent_mut.insert(McEntity::with_uuid(EntityKind::Player, client_ent, uuid));

        // Spawn armor stand
        let equipment = Equipment::default();
        let mut mc_entity = McEntity::new(EntityKind::ArmorStand, instance_entity);
        mc_entity.set_position([0.0, 0.0, 0.0]);
        let armor_stand = app.world.spawn((mc_entity, equipment));
        let armor_stand = armor_stand.id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Set armor stand boots
        let mut equipments = app
            .world
            .get_mut::<Equipment>(armor_stand)
            .expect("should have Equipment component");
        let item = ItemStack::new(ItemKind::IronBoots, 1, None);
        let slot = EquipmentSlot::Boots;
        equipments.set(item.clone(), slot);
        let armor_stand_entity_id = VarInt(
            app.world
                .get_mut::<McEntity>(armor_stand)
                .expect("should have McEntity component")
                .protocol_id(),
        );

        // Assert packet
        app.update();
        let sent_packets = client_helper.collect_sent()?;
        if let S2cPlayPacket::SetEquipment(packet) = &sent_packets[0] {
            assert_eq!(
                packet,
                &SetEquipment {
                    entity_id: armor_stand_entity_id,
                    equipment: vec![EquipmentEntry {
                        slot: slot.into(),
                        item: Some(item)
                    }]
                }
            )
        }

        // Set up for next tick
        app.update();
        client_helper.clear_sent();

        // Remove boots and set main hand
        let mut equipments = app
            .world
            .get_mut::<Equipment>(armor_stand)
            .expect("should have Equipment component");
        equipments.remove(EquipmentSlot::Boots);
        let item = ItemStack::new(ItemKind::DiamondSword, 1, None);
        let slot = EquipmentSlot::MainHand;
        equipments.set(item.clone(), slot);

        // Assert new packets
        app.update();
        let sent_packets = client_helper.collect_sent()?;
        if let S2cPlayPacket::SetEquipment(packet) = &sent_packets[0] {
            assert_eq!(
                packet,
                &SetEquipment {
                    entity_id: armor_stand_entity_id,
                    equipment: vec![
                        EquipmentEntry {
                            slot: slot.into(),
                            item: Some(item)
                        },
                        EquipmentEntry {
                            slot: EquipmentSlot::Boots.into(),
                            item: None
                        },
                    ]
                }
            )
        }

        Ok(())
    }
}
