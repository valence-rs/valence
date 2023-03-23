use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

use valence::bevy_app::prelude::*;
use valence::config::ServerPlugin;

fn main() -> io::Result<()> {
    let mut app = App::new();

    app.add_plugin(ServerPlugin::new(()));

    let dot_graph = bevy_mod_debugdump::schedule_graph_dot(
        &mut app,
        CoreSchedule::Main,
        &bevy_mod_debugdump::schedule_graph::Settings {
            ambiguity_enable: false,
            ..Default::default()
        },
    );

    let mut child = Command::new("dot")
        .stdin(Stdio::piped())
        .arg("-Tsvg")
        .arg("-o")
        .arg("graph.svg")
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(dot_graph.as_bytes())?;
    }

    child.wait_with_output()?;

    Ok(())
}
