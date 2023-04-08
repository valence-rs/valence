//! # Unit Test Cookbook
//!
//! Setting up an `App` with a single client:
//! ```ignore
//! # use bevy_app::App;
//! # use valence::unit_test::util::scenario_single_client;
//! let mut app = App::new();
//! let (client_ent, mut client_helper) = scenario_single_client(&mut app);
//! ```
//!
//! Asserting packets sent to the client:
//! ```ignore
//! # use bevy_app::App;
//! # use valence::unit_test::util::scenario_single_client;
//! # use valence::client::Client;
//! # fn main() -> anyhow::Result<()> {
//! # let mut app = App::new();
//! # let (client_ent, mut client_helper) = scenario_single_client(&mut app);
//! # let client: &Client = app.world.get(client_ent).expect("client not found");
//! client.write_packet(&valence_protocol::packets::s2c::play::KeepAliveS2c { id: 0xdeadbeef });
//! client.write_packet(&valence_protocol::packets::s2c::play::KeepAliveS2c { id: 0xf00dcafe });
//!
//! let sent_packets = client_helper.collect_sent()?;
//! assert_packet_count!(sent_packets, 2, S2cPlayPacket::KeepAliveS2c(_));
//! assert_packet_order!(
//!     sent_packets,
//!     S2cPlayPacket::KeepAliveS2c(KeepAliveS2c { id: 0xdeadbeef }),
//!     S2cPlayPacket::KeepAliveS2c(KeepAliveS2c { id: 0xf00dcafe }),
//! );
//! # Ok(())
//! # }
//! ```
//!
//! Performing a Query without a system is possible, like so:
//! ```
//! # use bevy_app::App;
//! # use valence::instance::Instance;
//! # let mut app = App::new();
//! app.world.query::<&Instance>();
//! ```

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
    use valence_protocol::packet::S2cPlayPacket;

    use super::*;
    use crate::client::Client;
    use crate::component::Position;
    use crate::inventory::{Inventory, InventoryKind, OpenInventory};
    use crate::{assert_packet_count, assert_packet_order};

    /// The server's tick should increment every update.
    #[test]
    fn example_test_server_tick_increment() {
        let mut app = App::new();
        app.add_plugin(ServerPlugin::new(()));
        let server = app.world.resource::<Server>();
        let tick = server.current_tick();
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
        let packet = valence_protocol::packet::c2s::play::PositionAndOnGround {
            position: [12.0, 64.0, 0.0],
            on_ground: true,
        };
        client_helper.send(&packet);

        // Process the packet.
        app.update();

        // Make assertions
        let pos = app.world.get::<Position>(client_ent).unwrap();
        assert_eq!(pos.0, [12.0, 64.0, 0.0].into());
    }

    /// A unit test where we want to test what packets are sent to the client.
    #[test]
    fn example_test_open_inventory() {
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
        let sent_packets = client_helper.collect_sent();

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::OpenScreenS2c(_));
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::InventoryS2c(_));
        assert_packet_order!(
            sent_packets,
            S2cPlayPacket::OpenScreenS2c(_),
            S2cPlayPacket::InventoryS2c(_)
        );
    }
}
