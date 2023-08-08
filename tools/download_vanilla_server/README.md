This is a simple script to download the vanilla Minecraft server from Mojang.
The server is configured to be easy to use with the packet inspector so you don't have to mess with settings.

# Running

```bash
cargo r -p download_vanilla_server
```

This will create `vanilla-server` in the current directory. `cd` into the folder and then run

```bash
java -jar server.jar nogui
```

to start the server. You can then run

```bash
cargo r -p packet_inspector
```

in another terminal window to start the packet inspector.
