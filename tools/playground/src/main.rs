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

use clap::Parser;
use tracing::Level;
use valence::{app::App, log::LogPlugin};

#[allow(dead_code)]
mod extras;
mod playground;

#[derive(Parser)]
struct Args {
    #[arg(short, default_value_t = Level::DEBUG)]
    log_level: Level,
}

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    app.add_plugin(LogPlugin {
        level: args.log_level,
        ..Default::default()
    });

    playground::build_app(&mut app);

    app.run();
}
