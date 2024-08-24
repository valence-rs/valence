package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.Codec;
import com.mojang.serialization.JsonOps;
import net.minecraft.registry.*;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

import java.util.stream.Stream;

public class PacketRegistries implements Main.Extractor  {

//    private static final RegistryOps<JsonElement> REGISTRY_OPS= RegistryOps.of(JsonOps.INSTANCE, DynamicRegistryManager.of(Regist));
    private final DynamicRegistryManager.Immutable registryManager;
    private final CombinedDynamicRegistries<ServerDynamicRegistryType> registries;

    public PacketRegistries(final MinecraftServer server) {
        registryManager = server.getRegistryManager();
        registries = server.getCombinedDynamicRegistries();
    }

    public String fileName() {
        return "registry_codec.json";
    }

    public static <T> JsonObject mapJson(final RegistryLoader.Entry<T> registry_entry, final DynamicRegistryManager.Immutable registryManager, final CombinedDynamicRegistries<ServerDynamicRegistryType> combinedRegistries) {
        final Codec<T> codec = registry_entry.elementCodec();
        final Registry<T> registry = registryManager.get(registry_entry.key());
        final JsonObject json = new JsonObject();
        registry.streamEntries().forEach(entry -> {
            json.add(entry.getKey().orElseThrow().getValue().toString(), codec.encodeStart(combinedRegistries.getCombinedRegistryManager().getOps(JsonOps.INSTANCE), entry.value()).resultOrPartial((e) -> Main.LOGGER.error("Cannot encode json: {}", e)).orElseThrow());
        });
        return json;
    }

    public JsonElement extract() {
        final Stream<RegistryLoader.Entry<?>> registries = RegistryLoader.SYNCED_REGISTRIES.stream();
        final JsonObject json = new JsonObject();
        registries.forEach(entry -> {
            json.add(entry.key().getValue().toString(), PacketRegistries.mapJson(entry, this.registryManager, this.registries));
        });
        return json;
    }
}
