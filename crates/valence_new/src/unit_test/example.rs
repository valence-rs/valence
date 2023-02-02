use bevy_app::App;

use crate::config::ServerPlugin;
use crate::server::Server;
use crate::unit_test::util::scenario_single_client;

/// Examples of valence unit tests that need to test the behavior of the server,
/// and not just the logic of a single function. This module is meant to be a
/// pallette of examples for how to write such tests, with various levels of
/// complexity.
///
/// Some of the tests in this file may be inferior duplicates of real tests.
#[cfg(test)]
mod tests {
    use valence_protocol::packets::S2cPlayPacket;

    use super::*;
    use crate::client::Client;
    use crate::inventory::{Inventory, InventoryKind, OpenInventory};
    use crate::{assert_packet_count, assert_packet_order};

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
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        // Send a packet as the client to the server.
        let packet = valence_protocol::packets::c2s::play::SetPlayerPosition {
            position: [12.0, 64.0, 0.0],
            on_ground: true,
        };
        client_helper.send(&packet);

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
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let inventory = Inventory::new(InventoryKind::Generic3x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();
        app.update();

        // Make assertions
        app.world
            .get::<Client>(client_ent)
            .expect("client not found");
        let sent_packets = client_helper.collect_sent()?;

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::OpenScreen(_));
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::SetContainerContent(_));
        assert_packet_order!(
            sent_packets,
            S2cPlayPacket::OpenScreen(_),
            S2cPlayPacket::SetContainerContent(_)
        );

        Ok(())
    }
}
