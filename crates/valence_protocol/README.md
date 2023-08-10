# valence_protocol

A protocol library for _Minecraft: Java Edition_. Use this to build clients, servers, proxies, or something novel!

`valence_protocol` is primarily concerned with defining all of Minecraft's [network packets](packets) and the process for encoding and decoding them. To encode and decode packets, use the [`PacketEncoder`] and [`PacketDecoder`] types.

```rust
use valence_protocol::{PacketEncoder, PacketDecoder, Difficulty};
use valence_protocol::packets::play::DifficultyS2c;

let mut encoder = PacketEncoder::new();

let packet = DifficultyS2c {
    difficulty: Difficulty::Peaceful,
    locked: true,
};

// Encode our packet struct.
encoder.append_packet(&packet);

// Take our encoded packet(s) out of the encoder.
let bytes = encoder.take();

let mut decoder = PacketDecoder::new();

// Put it in the decoder.
decoder.queue_bytes(bytes);

// Get the next packet "frame" from the decoder and use that to decode the body of the packet.
// Packet frames can be thought of as type-erased packet structs.
let frame = decoder.try_next_packet().unwrap().unwrap();
let decoded_packet = frame.decode::<DifficultyS2c>().unwrap();

// Check that our original packet struct is the same as the one we just decoded.
assert_eq!(&packet, &decoded_packet);
```

## Supported Minecraft Versions

Currently, `valence_protocol` only intends to support the most recent stable version of Minecraft. New Minecraft versions often entail a major version bump, since breaking changes to packet definitions are frequent.

The currently targeted Minecraft version and protocol version can be checked using the [`MINECRAFT_VERSION`] and [`PROTOCOL_VERSION`] constants.

## Feature Flags

- `encryption`: Enables support for packet encryption.
- `compression`: Enables support for packet compression.
