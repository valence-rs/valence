# valence_biome

Contains biomes and the biome registry. Minecraft's default biomes are added to the registry by default.

### **NOTE:**
- Modifying the biome registry after the server has started can
break invariants within instances and clients! Make sure there are no
instances or clients spawned before mutating.
- A biome named "minecraft:plains" must exist. Otherwise, vanilla clients
  will be disconnected.
