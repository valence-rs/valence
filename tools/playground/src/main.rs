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

use std::str::FromStr;
use std::env;

use tracing::Level;
use valence::app::App;

#[allow(dead_code)]
mod extras;
mod playground;

fn main() {
    let mut args = env::args().skip(1);
    let log_level = match (args.next().as_deref(), args.next()) {
        (Some("-l" | "--log"), Some(level)) => Level::from_str(&level).unwrap_or(Level::DEBUG),
        _ => Level::DEBUG,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    let mut app = App::new();
    playground::build_app(&mut app);
    app.run();
}
