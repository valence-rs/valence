use bevy_app::App;
use bevy_ecs::prelude::*;

use crate::config::ServerPlugin;
use crate::server::Server;

/// Examples of valence unit tests that need to test the behavior of the server,
/// and not just the logic of a single function. This module is meant to be a
/// pallette of examples for how to write such tests, with various levels of
/// complexity.
///
/// Some of the tests in this file may be inferior duplicates of real tests.
#[cfg(test)]
mod tests {
    use super::*;

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
}
