use core::time::Duration;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use args::StresserArgs;
use clap::Parser;
use stresser::make_session;
use tokio::sync::Semaphore;

mod args;
pub mod stresser;

#[tokio::main]
async fn main() {
    let args = StresserArgs::parse();

    let target_addr = args.target_host.to_socket_addrs().unwrap().next().unwrap();

    let mut session_index: usize = 0;

    let sema = Arc::new(Semaphore::new(args.connections_count));

    while let Ok(perm) = sema.clone().acquire_owned().await {
        let session_name = format!("{}{}", args.name_prefix, session_index);

        tokio::spawn(async move {
            if let Err(err) = make_session(target_addr, session_name.as_str()).await {
                eprintln!("Session {session_name} interrupted with error: {err}")
            };

            drop(perm);
        });

        session_index += 1;

        tokio::time::sleep(Duration::from_millis(args.spawn_cooldown)).await
    }
}
