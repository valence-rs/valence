use bevy_app::Plugin;
use bevy_ecs::{system::Query, query::Added};
use components::BossBarViewers;
use packet::{BossBarS2c, BossBarAction};
use valence_client::Client;
use valence_core::{despawn::Despawned, protocol::encode::WritePacket, uuid::UniqueId};

pub mod packet;
pub mod components;

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {

    fn build(&self, app: &mut bevy_app::App) {
        app
        .add_system(remove_despawned_boss_bars_from_viewers);
    }

}

fn remove_despawned_boss_bars_from_viewers(mut boss_bars: Query<(&UniqueId, &mut BossBarViewers), Added<Despawned>>, mut clients: Query<&mut Client>) {
    for boss_bar in boss_bars.iter_mut() {
        let (id, mut viewers) = boss_bar;

        for viewer in viewers.0.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&BossBarS2c {
                id: id.0,
                action: BossBarAction::Remove,
            });
        }
    }
}