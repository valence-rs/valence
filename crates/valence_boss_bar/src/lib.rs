use bevy_app::Plugin;
use bevy_ecs::{system::Query, query::Added};
use components::{BossBar, BossBarViewers};
use valence_client::Client;
use valence_core::{despawn::Despawned, protocol::encode::WritePacket};

pub mod packet;
pub mod components;

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {

    fn build(&self, app: &mut bevy_app::App) {
        app
        .add_system(remove_despawned_boss_bars_from_viewers);
    }

}

fn remove_despawned_boss_bars_from_viewers(mut bossbar_viewers: Query<(&BossBar, &mut BossBarViewers), Added<Despawned>>, mut clients: Query<&mut Client>) {
    for (boss_bar, mut viewers) in bossbar_viewers.iter_mut() {
        let viewers = &mut viewers.0;
        for viewer in viewers.iter_mut() {
            let mut client = clients.get_mut(*viewer).unwrap();
            client.write_packet(&boss_bar.generate_remove_packet());
        }
    }
}