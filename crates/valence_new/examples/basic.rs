use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use valence_new::config::Config;
use valence_new::server::Server;

fn main() -> anyhow::Result<()> {
    valence_new::run_server(
        Config::default(),
        SystemStage::single_threaded().with_system(do_this_once.with_run_criteria(ShouldRun::once)),
        (),
    )
}

fn do_this_once() {
    println!("this was done once!");
}


