use std::net::ToSocketAddrs;

use args::StresserArgs;
use clap::Parser;
use stresser::make_connection;

mod args;
pub mod stresser;

fn main() {
    let args = StresserArgs::parse();

    let target_addr = args.target_host.to_socket_addrs().unwrap().next().unwrap();

    make_connection(target_addr, args.name_prefix.as_str());
}
