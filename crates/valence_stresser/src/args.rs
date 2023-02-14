use clap::{arg, command, Parser};

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct StresserArgs {
    /// Represents an IPv4/IPv6/DNS address of a server.
    #[arg(short = 't', long = "host")]
    target_host: String,

    /// Represents a port of the target host.
    #[arg(short = 'p', long = "port")]
    target_port: u16,

    /// Represents an amount of connections to the target host.
    #[arg(short = 'c', long = "count")]
    connections_count: usize,
}
