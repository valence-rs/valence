# valence_chat

Provides support for cryptographically verified chat messaging on the server.

This crate contains the secure chat plugin as well as chat types and the chat type registry. Minecraft's default chat types are added to the registry by default. Chat types contain information about how chat is styled, such as the chat color.


This crate also contains the `yggdrasil_session_pubkey.der` file which is an encoded format of Mojang's public key. This is necessary to verify the integrity of our clients' public session key, which is used for validating chat messages. In reality Mojang's key should never change in order to maintain backwards compatibility with older versions, but if it does it can be extracted from any minecraft server jar.