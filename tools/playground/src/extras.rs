//! Put stuff in here if you find that you have to write the same code for
//! multiple playgrounds.

use valence::prelude::*;

/// Toggles client's game mode between survival and creative when they start
/// sneaking.
pub(crate) fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut GameMode>,
    mut events: EventReader<SneakEvent>,
) {
    for event in events.read() {
        if event.state == SneakState::Start {
            if let Ok(mut mode) = clients.get_mut(event.client) {
                *mode = match *mode {
                    GameMode::Survival => GameMode::Creative,
                    GameMode::Creative => GameMode::Survival,
                    _ => GameMode::Creative,
                };
            }
        }
    }
}
