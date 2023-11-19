#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    clippy::dbg_macro
)]

use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::Parser;
use valence::prelude::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Name of the schedule to dump. If absent, the list of available
    /// schedules is printed to stdout.
    schedule: Option<String>,
    /// Output SVG file path.
    #[clap(short, long, default_value = "graph.svg")]
    output: PathBuf,
    /// Disables transitive reduction of the output schedule graph.
    #[clap(short = 't', long)]
    no_tred: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    let schedules = app.world.resource::<Schedules>();

    let Some(sched_name) = cli.schedule else {
        print_available_schedules(schedules);
        return Ok(());
    };

    let Some((_, schedule)) = schedules
        .iter()
        .find(|(label, _)| format!("{label:?}") == sched_name)
    else {
        eprintln!("Unknown schedule \"{sched_name}\"");
        print_available_schedules(schedules);
        std::process::exit(1)
    };

    // let label = label.dyn_clone();

    let dot_graph = bevy_mod_debugdump::schedule_graph::schedule_graph_dot(
        schedule,
        &app.world,
        &bevy_mod_debugdump::schedule_graph::Settings {
            ambiguity_enable: false,
            ..Default::default()
        },
    );

    let mut dot_command = Command::new("dot");
    dot_command.arg("-Tsvg").arg("-o").arg(cli.output);

    if cli.no_tred {
        let mut dot_child = dot_command.stdin(Stdio::piped()).spawn()?;

        dot_child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(dot_graph.as_bytes())?;

        dot_child.wait_with_output()?;
    } else {
        let tred_child = Command::new("tred")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let dot_child = dot_command.stdin(tred_child.stdout.unwrap()).spawn()?;

        tred_child.stdin.unwrap().write_all(dot_graph.as_bytes())?;

        dot_child.wait_with_output()?;
    };

    Ok(())
}

fn print_available_schedules(schedules: &Schedules) {
    eprintln!("==== Available Schedules ====");

    for (label, _) in schedules.iter() {
        println!("{label:?}");
    }

    eprintln!("\nSee `--help` for more information.");
}
