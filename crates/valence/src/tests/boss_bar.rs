use bevy_app::App;
use bevy_ecs::entity::Entity;
use valence_boss_bar::components::{BossBar, BossBarColor, BossBarDivision, BossBarFlags};
use valence_core::text::Text;
use valence_instance::Instance;

use super::{MockClientHelper, scenario_single_client, create_mock_client};

/*#[test]
fn test_intialize_on_join() {
    let mut app = App::new();
    let (_, instance_ent) = prepare(&mut app);

    let (mut client, mut client_helper) = create_mock_client();
    let client_ent = app.world.spawn(client).id();

}*/

fn prepare(app: &mut App) -> (MockClientHelper, Entity) {
    let (_, mut client_helper) = scenario_single_client(app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_sent();

    // Get the instance entity.
    let instance_ent = app
        .world
        .iter_entities()
        .find(|e| e.contains::<Instance>())
        .expect("could not find instance")
        .id();

    // Insert a boss bar to the instance.
    app.world.entity_mut(instance_ent).insert(
            BossBar::new(Text::text(""),
                BossBarColor::Red,
                BossBarDivision::SixNotches,
                BossBarFlags::new()));
    
    for _ in 0..2 {
        app.update();
    }

    client_helper.clear_sent();
    (client_helper, instance_ent)
}
