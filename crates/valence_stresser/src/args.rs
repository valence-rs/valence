use clap::{arg, command, Parser};

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct StresserArgs {
    /// IPv4/IPv6/DNS address of a server.
    #[arg(short = 't', long = "target")]
    pub target_host: String,

    /// Port of the target host.
    #[arg(short = 'p', long = "port")]
    #[arg(default_value_t = 25565)]
    pub target_port: u16,

    /// Number of connections to the target.
    #[arg(short = 'c', long = "count")]
    pub connections_count: usize,
}
