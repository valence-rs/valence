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
use std::process::{Command, Stdio};

use valence::prelude::*;

fn main() -> io::Result<()> {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

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
