use core::time::Duration;
use std::net::ToSocketAddrs;
use std::thread::{self, JoinHandle};

use args::StresserArgs;
use clap::Parser;
use stresser::make_connection;

mod args;
pub mod stresser;

fn main() {
    let args = StresserArgs::parse();

    let target_addr = args.target_host.to_socket_addrs().unwrap().next().unwrap();

    let mut last_thread: Option<JoinHandle<()>> = None;

    for conn_index in 0..args.connections_count {
        let conn_name = format!("{}{}", args.name_prefix, conn_index);

        last_thread = Some(thread::spawn(move || {
            make_connection(target_addr, &conn_name.as_str());
        }));

        println!("Connections spawned: {}", conn_index + 1);

        thread::sleep(Duration::from_millis(args.spawn_cooldown))
    }

    if let Some(thread) = last_thread {
        _ = thread.join();
    }
}
