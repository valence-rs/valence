use clap::Parser;
use packet_inspector::Packet;
use packet_inspector::Proxy;
use packet_inspector::ProxyLog;
use packet_inspector::DisconnectionReason;
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

    let proxy = Proxy::start(args.listener_addr, args.server_addr).await?;
    let receiver = proxy.subscribe().await;

    tokio::spawn(async move {
        proxy.main_task.await??;
        Ok::<(), anyhow::Error>(())
    });

    // consumer
    tokio::spawn(async move {
        while let Ok(packet) = receiver.recv_async().await {
            log(&packet);
        }
    });

    tokio::spawn(async move {
        loop {
            let next = proxy.logs_rx.recv_async().await?;
            match next {
                ProxyLog::ClientConnected(addr) => {
                    tracing::trace!("Accepted a new client {addr}.");
                }
                ProxyLog::ClientDisconnected(addr, DisconnectionReason::Error(_)) => {
                    tracing::trace!("Client {addr} disconnected.");
                }
                ProxyLog::ClientDisconnected(addr, DisconnectionReason::OnlineModeRequired) => {
                    tracing::error!(
                        "Client {addr} was disconnected due to a server encryption request. \
                        The packet inspector does not support online mode."
                    );
                }
            }
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
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
