# `valence_network`

The plugin responsible for accepting connections and spawning clients.

This covers everything in the "handshaking", "status" and "login" stages of the protocol, before the main "play" stage begins. Support for proxies like [Velocity] and [BungeeCord] are implemented here.

Valence users can choose not to include `valence_network` in their project. This could be useful for testing or using Valence as an integrated server in a client.

[Velocity]: https://papermc.io/software/velocity
[BungeeCord]: https://github.com/SpigotMC/BungeeCord
