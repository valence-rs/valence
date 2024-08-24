package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import net.minecraft.entity.decoration.painting.PaintingVariant;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Paintings implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;

    public Paintings(final MinecraftServer server) {
        registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "paintings.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        final var dataDrivenMiscJson = new JsonObject();

        final var paintingRegistry = this.registryManager.get(RegistryKeys.PAINTING_VARIANT);

        final var codec = PaintingVariant.CODEC;

        final JsonObject json = new JsonObject();
        paintingRegistry.streamEntries().forEach(entry -> {
            json.add(entry.getKey().orElseThrow().getValue().toString(), codec.encodeStart(RegistryOps.of(JsonOps.INSTANCE, this.registryManager), entry.value()).getOrThrow());
        });

        return json;
    }
}
