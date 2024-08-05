use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::{ServerListLegacyPing, SharedNetworkState};

/// The payload of the legacy server list ping.
#[derive(PartialEq, Debug, Clone)]
pub enum ServerListLegacyPingPayload {
    /// The 1.6 legacy ping format, which includes additional data.
    Pre1_7 {
        /// The protocol version of the client.
        protocol: i32,
        /// The hostname the client used to connect to the server.
        hostname: String,
        /// The port the client used to connect to the server.
        port: u16,
    },
    /// The 1.4-1.5 legacy ping format.
    Pre1_6,
    /// The Beta 1.8-1.3 legacy ping format.
    Pre1_4,
}

/// Response data of the legacy server list ping.
///
/// # Example
///
/// ```
/// # use valence_network::ServerListLegacyPingResponse;
/// let mut response =
///     ServerListLegacyPingResponse::new(127, 0, 10).version("Valence 1.20.1".to_owned());
///
/// // This will make the description just repeat "hello" until the length limit
/// // (which depends on the other fields that we set above: protocol, version,
/// // online players, max players).
/// let max_description = response.max_description();
/// response = response.description(
///     std::iter::repeat("hello ")
///         .flat_map(|s| s.chars())
///         .take(max_description)
///         .collect(),
/// );
/// ```
#[derive(Clone, Default, Debug, PartialEq)]
pub struct ServerListLegacyPingResponse {
    protocol: i32,
    version: String,
    online_players: i32,
    max_players: i32,
    description: String,
}

#[derive(PartialEq, Debug)]
enum PingFormat {
    Pre1_4, // Beta 1.8 to 1.3
    Pre1_6, // 1.4 to 1.5
    Pre1_7, // 1.6
}

/// Returns true if legacy ping detected and handled
pub(crate) async fn try_handle_legacy_ping(
    shared: &SharedNetworkState,
    stream: &mut TcpStream,
    remote_addr: SocketAddr,
) -> io::Result<bool> {
    let mut temp_buf = [0_u8; 3];
    let mut n = stream.peek(&mut temp_buf).await?;

    if let [0xfe] | [0xfe, 0x01] = &temp_buf[..n] {
        // This could mean one of following things:
        // 1. The beginning of a normal handshake packet, not fully received yet though
        // 2. The beginning of the 1.6 legacy ping, not fully received yet either
        // 3. Pre-1.4 legacy ping (0xfe) or 1.4-1.5 legacy ping (0xfe 0x01), fully
        //    received
        //
        // So in the name of the Father, the Son, and the Holy Spirit, we pray,
        // and wait for more data to arrive if it's 1 or 2, and if no
        // data arrives for long enough, we can assume its 3.
        //
        // Downsides of this approach and where this could go wrong:
        // 1. Short artificial delay for pre-1.4 and 1.4-1.5 legacy pings
        // 2. If a normal handshake is encountered with the exact length of 0xfe 0x01 in
        //    VarInt format (extremely rare, the server address would have to be ~248
        //    bytes long), and for some God-forsaken reason sent the first 2 bytes of
        //    the packet but not any more in this whole time, we would incorrectly
        //    assume that it's a legacy ping and send an incorrect response.
        // 3. If it was a 1.6 legacy ping, but even after the delay we only received
        //    only 1 byte, then we would also send an incorrect response, thinking its a
        //    pre-1.4 ping. The client would still understand it though, it'd just think
        //    that the server is old (pre-1.4).
        //
        // In my opinion, 1 is insignificant, and 2/3 are so rare that they are
        // effectively insignificant too. Network IO is just not that reliable
        // at this level, the connection may be lost as well or something at this point.
        sleep(Duration::from_millis(10)).await;
        n = stream.peek(&mut temp_buf).await?;
    }

    let format = match &temp_buf[..n] {
        [0xfe] => PingFormat::Pre1_4,
        [0xfe, 0x01] => PingFormat::Pre1_6,
        [0xfe, 0x01, 0xfa] => PingFormat::Pre1_7,
        _ => return Ok(false), // Not a legacy ping
    };

    let payload = match format {
        PingFormat::Pre1_7 => read_payload(stream).await?,
        PingFormat::Pre1_6 => ServerListLegacyPingPayload::Pre1_6,
        PingFormat::Pre1_4 => ServerListLegacyPingPayload::Pre1_4,
    };

    if let ServerListLegacyPing::Respond(mut response) = shared
        .0
        .callbacks
        .inner
        .server_list_legacy_ping(shared, remote_addr, payload)
        .await
    {
        if format == PingFormat::Pre1_4 {
            // remove formatting for pre-1.4 legacy pings
            remove_formatting(&mut response.description);
        }

        let separator = match format {
            PingFormat::Pre1_4 => '§',
            _ => '\0',
        };

        let mut buf = Vec::new();

        // packet ID and length placeholder
        buf.extend([0xff, 0x00, 0x00]);

        if format != PingFormat::Pre1_4 {
            // some constant bytes lol
            buf.extend("§1\0".encode_utf16().flat_map(|c| c.to_be_bytes()));

            // protocol and version
            buf.extend(
                format!(
                    "{protocol}{separator}{version}{separator}",
                    protocol = response.protocol,
                    version = response.version
                )
                .encode_utf16()
                .flat_map(|c| c.to_be_bytes()),
            );
        }

        // Description
        buf.extend(
            response
                .description
                .encode_utf16()
                .flat_map(|c| c.to_be_bytes()),
        );

        // Online and max players
        buf.extend(
            format!(
                "{separator}{online_players}{separator}{max_players}",
                online_players = response.online_players,
                max_players = response.max_players
            )
            .encode_utf16()
            .flat_map(|c| c.to_be_bytes()),
        );

        // replace the length placeholder with the actual length
        let chars = (buf.len() as u16 - 3) / 2; // -3 because of the packet prefix (id and length), and /2 because UTF16
        buf[1..3].copy_from_slice(chars.to_be_bytes().as_slice());

        stream.write_all(&buf).await?;
    }

    Ok(true)
}

// Reads the payload of a 1.6 legacy ping
async fn read_payload(stream: &mut TcpStream) -> io::Result<ServerListLegacyPingPayload> {
    // consume the first 29 useless bytes of this amazing protocol
    stream.read_exact(&mut [0_u8; 29]).await?;

    let protocol = i32::from(stream.read_u8().await?);
    let hostname_len = usize::from(stream.read_u16().await?) * 2;

    if hostname_len > 512 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "hostname too long",
        ));
    }

    let mut hostname = vec![0_u8; hostname_len];
    stream.read_exact(&mut hostname).await?;
    let hostname = String::from_utf16_lossy(
        &hostname
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    );

    let port = stream.read_i32().await? as u16;

    Ok(ServerListLegacyPingPayload::Pre1_7 {
        protocol,
        hostname,
        port,
    })
}

impl ServerListLegacyPingResponse {
    const MAX_VALID_LENGTH: usize = 248;

    // Length of all the fields combined in string form. Used for validating and
    // comparing with MAX_VALID_LENGTH.
    fn length(&self) -> usize {
        let mut len = 0;
        len += int_len(self.protocol);
        len += int_len(self.online_players);
        len += int_len(self.max_players);
        len += self.version.encode_utf16().count();
        len += self.description.encode_utf16().count();

        len
    }
    /// Constructs a new basic [`ServerListLegacyPingResponse`].
    ///
    /// See [`description`][Self::description] and [`version`][Self::version].
    pub fn new(protocol: i32, online_players: i32, max_players: i32) -> Self {
        Self {
            protocol,
            version: String::new(),
            online_players,
            max_players,
            description: String::new(),
        }
    }
    /// Sets the description of the server.
    ///
    /// If the resulting response packet is too long to be valid, the
    /// description will be truncated.
    ///
    /// Use [`max_description`][Self::max_description] method to get the max
    /// valid length for this specific packet with the already set fields
    /// (version, protocol, online players, max players).
    ///
    /// Also any null bytes will be removed.
    pub fn description(mut self, description: String) -> Self {
        self.description = description;

        self.description.retain(|c| c != '\0');

        let overflow = self.length() as i32 - Self::MAX_VALID_LENGTH as i32;
        if overflow > 0 {
            let truncation_index = self
                .description
                .char_indices()
                .nth(self.description.encode_utf16().count() - overflow as usize)
                .unwrap()
                .0;
            self.description.truncate(truncation_index);
        }

        self
    }
    /// Sets the version of the server.
    ///
    /// If the resulting response packet is too long to be valid, the
    /// version will be truncated.
    ///
    /// Use [`max_version`][Self::max_version] method to get the max valid
    /// length for this specific packet with the already set fields
    /// (description, protocol, online players, max players).
    ///
    /// Also any null bytes will be removed.
    pub fn version(mut self, version: String) -> Self {
        self.version = version;

        self.version.retain(|c| c != '\0');

        let overflow = self.length() as i32 - Self::MAX_VALID_LENGTH as i32;
        if overflow > 0 {
            let truncation_index = self
                .version
                .char_indices()
                .nth(self.version.encode_utf16().count() - overflow as usize)
                .unwrap()
                .0;
            self.version.truncate(truncation_index);
        }

        self
    }
    /// Returns the maximum number of characters (not bytes) that this packet's
    /// description can have with all other fields set as they are.
    pub fn max_description(&self) -> usize {
        Self::MAX_VALID_LENGTH - (self.length() - self.description.encode_utf16().count())
    }
    /// Returns the maximum number of characters (not bytes) that this packet's
    /// version can have with all other fields set as they are.
    pub fn max_version(&self) -> usize {
        Self::MAX_VALID_LENGTH - (self.length() - self.version.encode_utf16().count())
    }
}

// Returns the length of a string representation of a signed integer
fn int_len(num: i32) -> usize {
    let num_abs = f64::from(num.abs());

    if num < 0 {
        (num_abs.log10() + 2.0) as usize // because minus sign
    } else {
        (num_abs.log10() + 1.0) as usize
    }
}

// Removes all `§` and their modifiers, if any
fn remove_formatting(string: &mut String) {
    while let Some(pos) = string.find('§') {
        // + 2 because we know that `§` is 2 bytes
        if let Some(c) = string[(pos + 2)..].chars().next() {
            // remove next char too if any
            string.replace_range(pos..(pos + 2 + c.len_utf8()), "");
        } else {
            string.remove(pos);
        }
    }
}
