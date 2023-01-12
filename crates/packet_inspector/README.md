# What's This?

The packet inspector is a very simple Minecraft proxy for viewing the contents of packets as they are sent/received.
It uses Valence's protocol facilities to print packet contents.
This was made for three purposes:

- Check that packets between Valence and client are matching your expectations.
- Check that packets between vanilla server and client are parsed correctly by Valence.
- Understand how the protocol works between the vanilla server and client.

# Usage

First, start a server

```sh
cargo r -r --example conway
```

In a separate terminal, start the packet inspector.

```sh
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565
```

The client must connect to `localhost:25566`. You should see the packets in `stdout`.

The `-i` and `-e` flags accept a regex to filter packets according to their name. The `-i` regex includes matching
packets while the `-e` regex excludes matching packets.

For instance, if you only want to print the packets `Foo`, `Bar`, and `Baz`, you can use a regex such
as `^(Foo|Bar|Baz)$` with the `-i` flag.

```sh
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565 -i '^(Foo|Bar|Baz)$'
```

Packets are printed to `stdout` while errors are printed to `stderr`. If you only want to see errors in your terminal,
direct `stdout` elsewhere.

```sh
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565 > log.txt
```

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
cargo r -r -p packet_inspector -- 127.0.0.1:25566 127.0.0.1:25565
```

Open Minecraft and connect to `localhost:25566`.

Clean up

```
docker stop mc
docker rm mc
```
