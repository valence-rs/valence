# Packet Inspector

![packet inspector screenshot](https://raw.githubusercontent.com/valence-rs/valence/main/assets/packet-inspector.png)

The packet inspector is a Minecraft proxy for viewing the contents of packets as
they are sent/received. It uses Valence's protocol facilities to display packet
contents. This was made for three purposes:

- Check that packets between Valence and client are matching your expectations.
- Check that packets between vanilla server and client are parsed correctly by
  Valence.
- Understand how the protocol works between the vanilla server and client.

# Usage

Firstly, we should have a server running that we're going to be
proxying/inspecting.

```sh
cargo r -r --example game_of_life
```

Next up, we need to run the proxy server, this can be done in 2 different ways,
either using the GUI application (default) or by using the `cli` feature gate.

To launch in a Gui environment, simply run `packet_inspector`.

```sh
cargo r -r -p packet_inspector
```

To Launch in a Cli environment, build `packet_inspector` with the default
features disabled, and supplying the `cli` feature. note that you **must**
supply the listener and server addresses as arguments.

```bash
cargo r -r -p packet_inspector --no-default-features --features cli -- 127.0.0.1:25566 127.0.0.1:25565
```

To assist, `--help` will produce the following:

```
A simple Minecraft proxy for inspecting packets.

Usage: packet_inspector <LISTENER_ADDR> <SERVER_ADDR>

Arguments:
  <LISTENER_ADDR>  The socket address to listen for connections on. This is the address clients should connect to
  <SERVER_ADDR>    The socket address the proxy will connect to. This is the address of the server
```

The client can now connect to `localhost:25566`. You should see the packets in
`stdout` when running in cli mode, or you should see packets streaming in on the
Gui.

## Quick start with Vanilla Server via Docker

Start the server

```sh
docker run -e EULA=TRUE -e ONLINE_MODE=false -d -p 25565:25565 --name mc itzg/minecraft-server
```

View server logs

```sh
docker logs -f mc
```

Server Rcon

```sh
docker exec -i mc rcon-cli
```

In a separate terminal, start the packet inspector.

```sh
cargo r -r -p packet_inspector --no-default-features --features cli -- 127.0.0.1:25566 127.0.0.1:25565
```

Open Minecraft and connect to `localhost:25566`.

Clean up

```
docker stop mc
docker rm mc
```
