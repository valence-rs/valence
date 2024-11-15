package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.Codec;
import com.mojang.serialization.JsonOps;
import java.util.stream.Stream;
import net.minecraft.registry.*;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class PacketRegistries implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;
    private final CombinedDynamicRegistries<
        ServerDynamicRegistryType
    > registries;

    public PacketRegistries(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
        this.registries = server.getCombinedDynamicRegistries();
    }

    public String fileName() {
        return "registry_codec.json";
    }

    public static <T> JsonObject mapJson(
        RegistryLoader.Entry<T> registry_entry,
        DynamicRegistryManager.Immutable registryManager,
        CombinedDynamicRegistries<ServerDynamicRegistryType> combinedRegistries
    ) {
        Codec<T> codec = registry_entry.elementCodec();
        Registry<T> registry = registryManager.getOrThrow(registry_entry.key());
        JsonObject json = new JsonObject();
        registry
            .streamEntries()
            .forEach(entry -> {
                json.add(
                    entry.getKey().orElseThrow().getValue().toString(),
                    codec
                        .encodeStart(
                            combinedRegistries
                                .getCombinedRegistryManager()
                                .getOps(JsonOps.INSTANCE),
                            entry.value()
                        )
                        .resultOrPartial(e ->
                            Main.LOGGER.error("Cannot encode json: {}", e)
                        )
                        .orElseThrow()
                );
            });
        return json;
    }

    public JsonElement extract() {
        Stream<RegistryLoader.Entry<?>> registries =
            RegistryLoader.SYNCED_REGISTRIES.stream();
        JsonObject json = new JsonObject();
        registries.forEach(entry -> {
            json.add(
                entry.key().getValue().toString(),
                mapJson(entry, registryManager, this.registries)
            );
        });
        return json;
    }
}
