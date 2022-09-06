# What's This?

The packet inspector is a very simple Minecraft proxy for viewing the contents of packets as they are sent/received.
It uses Valence's protocol facilities to print packet contents.
This was made for two purposes:
- Check that packets between Valence and client are matching your expectations.
- Check that packets between vanilla server and client are parsed correctly by Valence.
- Understand how the protocol works between the vanilla server and client.

# Usage

First, start a server

```
cargo r -r --example conway
```
In a separate terminal, start the packet inspector. 

```sh
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565
```

The client must connect to `localhost:25566`. You should see the packets in `stdout`.

If you only want to see errors, direct `stderr` elsewhere.

```sh
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565 > log.txt
```
