use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

use valence_protocol::packets::c2s::handshake::Handshake;
use valence_protocol::packets::c2s::login::LoginStart;
use valence_protocol::packets::c2s::play::{ConfirmTeleport, KeepAliveC2s, SetPlayerPosition};
use valence_protocol::packets::{C2sHandshakePacket, S2cLoginPacket, S2cPlayPacket};
use valence_protocol::types::HandshakeNextState;
use valence_protocol::{PacketDecoder, PacketEncoder, Username, Uuid, VarInt};

// At higher values something going wrong and keep alive packets are not
// handling.
const BUFFER_SIZE: usize = 4;

pub fn make_connection(socket_addr: SocketAddr, connection_name: &str) {
    let mut conn = TcpStream::connect(socket_addr).unwrap();

    _ = conn.set_nodelay(true);

    let mut dec = PacketDecoder::new();
    let mut enc = PacketEncoder::new();

    let server_addr_str = socket_addr.ip().to_string().as_str().to_owned();

    let handshake_pkt = C2sHandshakePacket::Handshake(Handshake {
        protocol_version: VarInt::from(761),
        server_address: &server_addr_str,
        server_port: socket_addr.port(),
        next_state: HandshakeNextState::Login,
    });

    _ = enc.append_packet(&handshake_pkt);

    let write_buf = enc.take();

    _ = conn.write_all(&write_buf);

    enc.clear();

    _ = enc.append_packet(&LoginStart {
        username: Username::new(connection_name).unwrap(),
        profile_id: Some(Uuid::new_v4()),
    });

    let write_buf = enc.take();

    _ = conn.write_all(&write_buf);

    enc.clear();

    loop {
        let mut read_buf = [0 as u8; BUFFER_SIZE];
        let bytes_read = conn.read(&mut read_buf).unwrap();
        let bytes = &mut read_buf[..bytes_read];

        if bytes_read > 0 {
            println!("\nBytes read: {bytes_read}");

            dec.reserve(BUFFER_SIZE);
            dec.queue_slice(bytes);

            if let Ok(pkt) = dec.try_next_packet::<S2cLoginPacket>() {
                println!("Got login packet");

                match pkt {
                    Some(pkt) => match pkt {
                        S2cLoginPacket::SetCompression(p) => {
                            println!("Got set compression packet");

                            let threshold = p.threshold.0 as u32;

                            println!("Compression threshold: {}", threshold);

                            dec.set_compression(true);
                            enc.set_compression(Some(threshold));

                            println!("Compression enabled");
                        }

                        S2cLoginPacket::LoginSuccess(_) => {
                            println!("Logic success");
                            break;
                        }

                        _ => (),
                    },

                    None => (),
                }
            }
        }
    }

    loop {
        let mut read_buf = [0 as u8; BUFFER_SIZE];
        let bytes_read = conn.read(&mut read_buf).unwrap();
        let bytes = &mut read_buf[..bytes_read];

        if bytes_read > 0 {
            println!("\nBytes read: {bytes_read}");

            dec.reserve(BUFFER_SIZE);
            dec.queue_slice(bytes);

            match dec.try_next_packet::<S2cPlayPacket>() {
                Ok(pkt) => {
                    println!("Got play packet");

                    match pkt {
                        Some(pkt) => match pkt {
                            S2cPlayPacket::KeepAliveS2c(p) => {
                                enc.clear();
                                _ = enc.append_packet(&KeepAliveC2s { id: p.id });
                                _ = conn.write_all(&enc.take());

                                println!("Keep alive: {}", p.id);
                            }

                            S2cPlayPacket::SynchronizePlayerPosition(p) => {
                                enc.clear();
                                _ = enc.append_packet(&ConfirmTeleport {
                                    teleport_id: p.teleport_id,
                                });
                                _ = conn.write_all(&enc.take());

                                println!("Confirm teleport: {}", p.teleport_id.0);

                                enc.clear();
                                _ = enc.append_packet(&SetPlayerPosition {
                                    position: p.position,
                                    on_ground: true,
                                });
                                _ = conn.write_all(&enc.take());

                                println!("Set player position: {:?}", p.position);
                            }

                            S2cPlayPacket::ChunkDataAndUpdateLight(_) => {
                                println!("Ignore chunk data")
                            }

                            _ => println!("{pkt:?}"),
                        },
                        None => (),
                    }
                }
                Err(e) => println!("{e}"),
            }
        }
    }
}
