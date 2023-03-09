# What's This?

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
cargo r -r --example conway
```

Next up, we need to run the proxy server, this can be done in 2 different ways,
either using the GUI application (default) or using the `--nogui` flag to log
the packets to a terminal instance.

To assist, `--help` will produce the following:

```
A simple Minecraft proxy for inspecting packets.

Usage: packet_inspector [OPTIONS] [CLIENT_ADDR] [SERVER_ADDR]

Arguments:
  [CLIENT_ADDR]  The socket address to listen for connections on. This is the address clients should connect to
  [SERVER_ADDR]  The socket address the proxy will connect to. This is the address of the server

Options:
  -m, --max-connections <MAX_CONNECTIONS>
          The maximum number of connections allowed to the proxy. By default, there is no limit
      --nogui
          Disable the GUI. Logging to stdout
  -i, --include-filter <INCLUDE_FILTER>
          Only show packets that match the filter
  -e, --exclude-filter <EXCLUDE_FILTER>
          Hide packets that match the filter. Note: Only in effect if nogui is set
  -h, --help
          Print help
  -V, --version
          Print version
```

To launch in a Gui environment, simply launch `packet_inspector[.exe]` (or
`cargo r -r -p packet_inspector` to run from source). The gui will prompt you
for the `CLIENT_ADDR` and `SERVER_ADDR` if they have not been supplied via the
command line arguments.

In a terminal only environment, use the `--nogui` option and supply
`CLIENT_ADDR` and `SERVER_ADDR` as arguments.

```bash
cargo r -r -p packet_inspector -- --nogui 127.0.0.1:25566 127.0.0.1:25565
```

The client must connect to `localhost:25566`. You should see the packets in
`stdout` when running in `--nogui`, or you should see packets streaming in on
the Gui.

The `-i` and `-e` flags accept a regex to filter packets according to their
name. The `-i` regex includes matching packets while the `-e` regex excludes
matching packets. Do note that `-e` only applies in `--nogui` environment, as
the Gui has a "packet selector" to enable/disable packets dynamically. The `-i`
parameter value will be included in the `Filter` input field on the Gui.

For instance, if you only want to print the packets `Foo`, `Bar`, and `Baz`, you
can use a regex such as `^(Foo|Bar|Baz)$` with the `-i` flag.

```sh
cargo r -r -p packet_inspector -- --nogui 127.0.0.1:25566 127.0.0.1:25565 -i '^(Foo|Bar|Baz)$'
```

Packets are printed to `stdout` while errors are printed to `stderr`. If you
only want to see errors in your terminal, direct `stdout` elsewhere.

```sh
cargo r -r -p packet_inspector -- --nogui 127.0.0.1:25566 127.0.0.1:25565 > log.txt
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
cargo r -r -p packet_inspector -- --nogui 127.0.0.1:25566 127.0.0.1:25565
```

Open Minecraft and connect to `localhost:25566`.

Clean up

```
docker stop mc
docker rm mc
```
