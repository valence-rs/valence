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

    public Paintings(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "paintings.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var dataDrivenMiscJson = new JsonObject();

        // TODO - For the moment I don't know how does this registers works
        var paintingRegistry = registryManager.get(RegistryKeys.PAINTING_VARIANT);

        var codec = PaintingVariant.CODEC;

        JsonObject json = new JsonObject();
        paintingRegistry.streamEntries().forEach(entry -> {
            json.add(entry.getKey().orElseThrow().getValue().toString(), codec.encodeStart(RegistryOps.of(JsonOps.INSTANCE, registryManager), entry.value()).getOrThrow());
        });

        return json;
    }
}
