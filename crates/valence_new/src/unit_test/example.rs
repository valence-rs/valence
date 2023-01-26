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

    use super::*;
    use crate::client::Client;

    /// The server's tick should increment every update.
    #[test]
    fn test_server_tick_increment() {
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
    #[tokio::test]
    async fn test_client_position() {
        let mut app = App::new();
        app.add_plugin(ServerPlugin::new(()));
        let server = app.world.resource::<Server>();
        let permit = server.force_aquire_owned();
        let info = gen_client_info("test");
        let (client, mut client_helper) = create_mock_client(permit, info);
        let client_ent = app.world.spawn(client).id();

        // Send a packet as the client to the server.
        let packet = valence_protocol::packets::c2s::play::SetPlayerPosition {
            position: [12.0, 64.0, 0.0],
            on_ground: true,
        };
        client_helper.send_packet(packet);

        // Process the packet.
        app.update();

        // Make assertions
        let client: &Client = app.world.get(client_ent).unwrap();
        assert_eq!(client.position(), [12.0, 64.0, 0.0].into());
    }
}
