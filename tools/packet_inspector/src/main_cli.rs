use clap::Parser;
use packet_inspector::Packet;
use packet_inspector::Proxy;
use std::net::SocketAddr;
use tracing::Level;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct CliArgs {
    /// The socket address to listen for connections on. This is the address clients should connect to
    listener_addr: SocketAddr,
    /// The socket address the proxy will connect to. This is the address of the server
    server_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let args = CliArgs::parse();

    let proxy = Proxy::new(args.listener_addr, args.server_addr);
    let receiver = proxy.subscribe();

    tokio::spawn(async move {
        proxy.run().await?;

        Ok::<(), anyhow::Error>(())
    });

    // consumer
    tokio::spawn(async move {
        while let Ok(packet) = receiver.recv_async().await {
            log(&packet);
        }
    });

    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}

fn log(packet: &Packet) {
    tracing::debug!(
        "{:?} -> [{:?}] 0x{:0>2X} \"{}\" {:?}",
        packet.side,
        packet.state,
        packet.id,
        packet.name,
        truncated(format!("{:?}", packet.data), 512)
    );
}

fn truncated(string: String, max_len: usize) -> String {
    if string.len() > max_len {
        format!("{}...", &string[..max_len])
    } else {
        string
    }
}
