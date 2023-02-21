use std::io::{self, ErrorKind};
use std::net::SocketAddr;

use anyhow::bail;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use valence_protocol::packets::c2s::handshake::Handshake;
use valence_protocol::packets::c2s::login::LoginStart;
use valence_protocol::packets::c2s::play::{ConfirmTeleport, KeepAliveC2s, SetPlayerPosition};
use valence_protocol::packets::{C2sHandshakePacket, S2cLoginPacket, S2cPlayPacket};
use valence_protocol::types::HandshakeNextState;
use valence_protocol::{PacketDecoder, PacketEncoder, Username, Uuid, VarInt, PROTOCOL_VERSION};

pub struct SessionParams<'a> {
    pub socket_addr: SocketAddr,
    pub session_name: &'a str,
    pub read_buffer_size: usize,
}

pub async fn make_session<'a>(params: &SessionParams<'a>) -> anyhow::Result<()> {
    let sock_addr = params.socket_addr;
    let sess_name = params.session_name;
    let rb_size = params.read_buffer_size;

    let mut conn = match TcpStream::connect(sock_addr).await {
        Ok(conn) => {
            println!("{sess_name} connected");
            conn
        }
        Err(err) => {
            println!("{sess_name} connection failed");
            return Err(err.into());
        }
    };

    _ = conn.set_nodelay(true);

    let mut dec = PacketDecoder::new();
    let mut enc = PacketEncoder::new();

    let server_addr_str = sock_addr.ip().to_string().as_str().to_owned();

    let handshake_pkt = C2sHandshakePacket::Handshake(Handshake {
        protocol_version: VarInt::from(PROTOCOL_VERSION),
        server_address: &server_addr_str,
        server_port: sock_addr.port(),
        next_state: HandshakeNextState::Login,
    });

    _ = enc.append_packet(&handshake_pkt);

    _ = enc.append_packet(&LoginStart {
        username: Username::new(sess_name).unwrap(),
        profile_id: Some(Uuid::new_v4()),
    });

    let write_buf = enc.take();
    conn.write_all(&write_buf).await?;

    loop {
        dec.reserve(rb_size);

        let mut read_buf = dec.take_capacity();

        conn.readable().await?;

        match conn.try_read_buf(&mut read_buf) {
            Ok(0) => return Err(io::Error::from(ErrorKind::UnexpectedEof).into()),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
            Ok(_) => (),
        };

        dec.queue_bytes(read_buf);

        if let Ok(Some(pkt)) = dec.try_next_packet::<S2cLoginPacket>() {
            match pkt {
                S2cLoginPacket::SetCompression(p) => {
                    let threshold = p.threshold.0 as u32;

                    dec.set_compression(true);
                    enc.set_compression(Some(threshold));
                }

                S2cLoginPacket::LoginSuccess(_) => {
                    break;
                }

                S2cLoginPacket::EncryptionRequest(_) => {
                    bail!("encryption not implemented");
                }

                _ => (),
            }
        }
    }

    println!("{sess_name} logined");

    loop {
        while !dec.has_next_packet()? {
            dec.reserve(rb_size);

            let mut read_buf = dec.take_capacity();

            conn.readable().await?;

            match conn.try_read_buf(&mut read_buf) {
                Ok(0) => return Err(io::Error::from(ErrorKind::UnexpectedEof).into()),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
                Ok(_) => (),
            };

            dec.queue_bytes(read_buf);
        }

        match dec.try_next_packet::<S2cPlayPacket>() {
            Ok(None) => continue,
            Ok(Some(pkt)) => match pkt {
                S2cPlayPacket::KeepAliveS2c(p) => {
                    enc.clear();

                    _ = enc.append_packet(&KeepAliveC2s { id: p.id });
                    conn.write_all(&enc.take()).await?;

                    println!("{sess_name} keep alive")
                }

                S2cPlayPacket::SynchronizePlayerPosition(p) => {
                    enc.clear();

                    _ = enc.append_packet(&ConfirmTeleport {
                        teleport_id: p.teleport_id,
                    });

                    _ = enc.append_packet(&SetPlayerPosition {
                        position: p.position,
                        on_ground: true,
                    });

                    conn.write_all(&enc.take()).await?;

                    println!("{sess_name} spawned")
                }
                _ => (),
            },
            Err(err) => return Err(err),
        }
    }
}
