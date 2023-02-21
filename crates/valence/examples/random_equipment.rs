use bevy_app::CoreStage;
use rand::Rng;
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::equipment::{Equipment, EquipmentSlot};
use valence::prelude::*;

const BOARD_MIN_X: i32 = -30;
const BOARD_MAX_X: i32 = 30;
const BOARD_MIN_Z: i32 = -30;
const BOARD_MAX_Z: i32 = 30;
const BOARD_Y: i32 = 64;

const SPAWN_POS: DVec3 = DVec3::new(
    (BOARD_MIN_X + BOARD_MAX_X) as f64 / 2.0,
    BOARD_Y as f64 + 1.0,
    (BOARD_MIN_Z + BOARD_MAX_Z) as f64 / 2.0,
);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_to_stage(CoreStage::Update, randomize_equipment)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -10..10 {
        for x in -10..10 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in BOARD_MIN_Z..=BOARD_MAX_Z {
        for x in BOARD_MIN_X..=BOARD_MAX_X {
            instance.set_block([x, BOARD_Y, z], BlockState::DIRT);
        }
    }

    let instance = world.spawn(instance);
    let instance_entity = instance.id();

    let mut equipment = Equipment::default();
    equipment.set(
        ItemStack::new(ItemKind::IronBoots, 1, None),
        EquipmentSlot::Boots,
    );

    // Spawn armor stand
    let mut armor_stand = world.spawn((
        McEntity::new(EntityKind::ArmorStand, instance_entity),
        equipment,
    ));

    if let Some(mut armor_stand) = armor_stand.get_mut::<McEntity>() {
        let position = [SPAWN_POS.x, SPAWN_POS.y, SPAWN_POS.z + 3.0];
        armor_stand.set_position(position);
        armor_stand.set_yaw(180.0);
    }
}

fn init_clients(
    mut clients: Query<(&mut Client, Entity), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    let instance = instances.single();

    for (mut client, entity) in &mut clients {
        client.set_position(SPAWN_POS);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Creative);

        let equipment = Equipment::default();

        commands.entity(entity).insert((
            equipment,
            McEntity::with_uuid(EntityKind::Player, instance, client.uuid()),
        ));
    }
}

fn randomize_equipment(mut query: Query<&mut Equipment>, server: Res<Server>) {
    let ticks = server.current_tick();
    if ticks % server.tps() != 0 {
        return;
    }

    for mut equips in &mut query {
        equips.clear();

        let (slot, item_kind) = match rand::thread_rng().gen_range(0..=5) {
            0 => (EquipmentSlot::MainHand, ItemKind::DiamondSword),
            1 => (EquipmentSlot::OffHand, ItemKind::Torch),
            2 => (EquipmentSlot::Boots, ItemKind::IronBoots),
            3 => (EquipmentSlot::Leggings, ItemKind::DiamondLeggings),
            4 => (EquipmentSlot::Chestplate, ItemKind::ChainmailChestplate),
            5 => (EquipmentSlot::Helmet, ItemKind::LeatherHelmet),
            _ => (EquipmentSlot::Boots, ItemKind::IronBoots),
        };

        let item = ItemStack::new(item_kind, 1, None);

        equips.set(item, slot);
    }
}
