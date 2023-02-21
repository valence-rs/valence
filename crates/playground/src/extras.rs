//! Put stuff in here if you find that you have to write the same code for
//! multiple playgrounds.

use valence::client::event::StartSneaking;
use valence::prelude::*;

/// Toggles client's game mode between survival and creative when they start
/// sneaking.
pub fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut Client>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        let mode = client.game_mode();
        client.set_game_mode(match mode {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        });
    }
}
