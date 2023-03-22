use valence::bevy_app::prelude::*;
use valence::config::ServerPlugin;

fn main() -> std::io::Result<()> {
    let mut app = App::new();

    app.add_plugin(ServerPlugin::new(()));

    let data = bevy_mod_debugdump::schedule_graph_dot(
        &mut app,
        CoreSchedule::Main,
        &bevy_mod_debugdump::schedule_graph::Settings {
            ambiguity_enable: false,
            ..Default::default()
        },
    );

    let path = "graph.gv";

    println!("Writing schedule dump to file '{path}'");

    std::fs::write(path, data)
}
