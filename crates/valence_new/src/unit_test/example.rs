use bevy_app::App;
use bevy_ecs::prelude::*;

use super::util::create_mock_client;
use crate::config::ServerPlugin;
use crate::server::Server;
use crate::unit_test::util::gen_client_info;

/// Examples of valence unit tests that need to test the behavior of the server,
/// and not just the logic of a single function. This module is meant to be a
/// pallette of examples for how to write such tests, with various levels of
/// complexity.
///
/// Some of the tests in this file may be inferior duplicates of real tests.
#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use valence_protocol::packets::S2cPlayPacket;

    use super::*;
    use crate::client::Client;
    use crate::dimension::DimensionId;
    use crate::inventory::{Inventory, InventoryKind, OpenInventory};

    /// The server's tick should increment every update.
    #[test]
    fn example_test_server_tick_increment() {
        let mut app = App::new();
        app.add_plugin(ServerPlugin::new(()));
        let server = app.world.resource::<Server>();
        let tick = server.current_tick();
        drop(server);
        app.update();
        let server = app.world.resource::<Server>();
        assert_eq!(server.current_tick(), tick + 1);
    }

    /// A unit test where we want to test what happens when a client sends a
    /// packet to the server.
    #[test]
    fn example_test_client_position() {
        let mut app = App::new();
        app.add_plugin(ServerPlugin::new(()));
        let server = app.world.resource::<Server>();
        let info = gen_client_info("test");
        let (client, mut client_helper) = create_mock_client(info);
        let client_ent = app.world.spawn(client).id();

        // Send a packet as the client to the server.
        let packet = valence_protocol::packets::c2s::play::SetPlayerPosition {
            position: [12.0, 64.0, 0.0],
            on_ground: true,
        };
        // client_helper.send_packet(packet);

        // Process the packet.
        app.update();

        // Make assertions
        let client: &Client = app.world.get(client_ent).expect("client not found");
        assert_eq!(client.position(), [12.0, 64.0, 0.0].into());
    }

    /// A unit test where we want to test what packets are sent to the client.
    #[test]
    fn example_test_open_inventory() -> anyhow::Result<()> {
        let mut app = App::new();
        app.add_plugin(ServerPlugin::new(()));
        let server = app.world.resource::<Server>();
        let instance = server.new_instance(DimensionId::default());
        let instance_ent = app.world.spawn(instance).id();
        let info = gen_client_info("test");
        let (mut client, mut client_helper) = create_mock_client(info);
        client.set_instance(instance_ent); // HACK: needed so client does not get disconnected on first update
        let inventory = Inventory::new(InventoryKind::Generic3x3);
        let inventory_ent = app.world.spawn(inventory).id();
        let client_ent = app
            .world
            .spawn((client, Inventory::new(InventoryKind::Player)))
            .id();

        // Process a tick to get past the "on join" logic.
        app.update();
        // let stream = client_helper.inner_stream();
        // let mut stream = stream.lock().unwrap();
        // stream.clear_sent();
        // drop(stream);

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();
        app.update();

        // Make assertions
        // let client: &Client = app.world.get(client_ent).expect("client not found");
        // let stream = client_helper.inner_stream();
        // let mut stream = stream.lock().unwrap();
        // let sent_packets = stream.collect_sent()?;
        let sent_packets = vec![];
        assert_eq!(sent_packets.len(), 2);

        let open_idx = sent_packets
            .iter()
            .position(|p| matches!(p, S2cPlayPacket::OpenScreen(_)))
            .expect("no OpenScreen packet sent");
        let container_idx = sent_packets
            .iter()
            .position(|p| matches!(p, S2cPlayPacket::SetContainerContent(_)))
            .expect("no SetContainerContent packet sent");
        assert!(open_idx < container_idx);

        Ok(())
    }
}
