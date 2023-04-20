# valence_registry

Manages Minecraft's networked registries in a generic way. This includes the registry codec sent to clients during the initial join.

Consumers of `registry` such as `biome` and `dimension` are expected to update themselves in the registries defined here. Minecraft's default registry codec is loaded by default.

End users are not expected to use this module directly.
