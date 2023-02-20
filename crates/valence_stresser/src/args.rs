use clap::{arg, command, Parser};

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct StresserArgs {
    /// IPv4/IPv6/DNS address of a server.
    #[arg(short = 't', long = "target")]
    pub target_host: String,

    /// Number of sessions.
    #[arg(short = 'c', long = "count")]
    pub connections_count: usize,

    /// Name prefix of sessions.
    #[arg(default_value = "Stresser")]
    #[arg(short = 'n', long = "name")]
    pub name_prefix: String,

    /// Spawn cooldown of sessions in milliseconds.
    /// The lower the value, the more frequently connections are spawned.
    #[arg(default_value = "10")]
    #[arg(long = "cooldown")]
    pub spawn_cooldown: u64,
}
