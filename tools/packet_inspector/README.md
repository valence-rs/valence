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

Next up, we need to run the proxy server, To launch in a GUI environment, simply run `packet_inspector`.

```sh
cargo r -r -p packet_inspector
```

Then click the "Start Listening" button in the top left of the UI.

The client can now connect to `localhost:25566`. You should see packets streaming in on the GUI.

## Quick start with Vanilla Server via Docker

Start the server

```sh
docker run -e EULA=TRUE -e ONLINE_MODE=false -e ANNOUNCE_PLAYER_ACHIEVEMENTS=false -e GENERATE_STRUCTURES=false -e SPAWN_ANIMALS=false -e SPAWN_MONSTERS=false -e SPAWN_NPCS=false -e SPAWN_PROTECTION=0 -e VIEW_DISTANCE=16 -e MODE=creative -e LEVEL_TYPE=flat -e RCON_CMDS_STARTUP="gamerule doWeatherCycle false" -d -p 25565:25565 --name mc itzg/minecraft-server
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
