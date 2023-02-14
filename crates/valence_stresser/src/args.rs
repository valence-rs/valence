use clap::{arg, command, Parser};

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct StresserArgs {
    /// IPv4/IPv6/DNS address of a server.
    #[arg(short = 't', long = "target")]
    target_host: String,

    /// Port of the target host.
    #[arg(short = 'p', long = "port")]
    target_port: u16,

    /// Number of connections to the target.
    #[arg(short = 'c', long = "count")]
    connections_count: usize,
}
