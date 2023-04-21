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

use core::time::Duration;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use args::StresserArgs;
use clap::Parser;
use stresser::{make_session, SessionParams};
use tokio::sync::Semaphore;

mod args;
pub mod stresser;

#[tokio::main]
async fn main() {
    let args = StresserArgs::parse();

    let target_addr = args.target_host.to_socket_addrs().unwrap().next().unwrap();

    let mut session_index: usize = 0;

    let sema = Arc::new(Semaphore::new(args.sessions_count));

    while let Ok(perm) = sema.clone().acquire_owned().await {
        let session_name = format!("{}{}", args.name_prefix, session_index);

        tokio::spawn(async move {
            let params = SessionParams {
                socket_addr: target_addr,
                session_name: session_name.as_str(),
                read_buffer_size: args.read_buffer_size,
            };

            if let Err(err) = make_session(&params).await {
                eprintln!("Session {session_name} interrupted with error: {err}")
            };

            drop(perm);
        });

        session_index += 1;

        tokio::time::sleep(Duration::from_millis(args.spawn_cooldown)).await;
    }
}
