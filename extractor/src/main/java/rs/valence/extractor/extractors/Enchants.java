package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import net.minecraft.enchantment.Enchantment;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Enchants implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;

    public Enchants(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "enchants.json";
    }

    @Override
    public JsonElement extract() {
        var enchantsJson = new JsonObject();

        for (var enchant : registryManager
            .getOrThrow(RegistryKeys.ENCHANTMENT)
            .streamEntries()
            .toList()) {
            enchantsJson.add(
                enchant.getKey().orElseThrow().getValue().toString(),
                Enchantment.CODEC.encodeStart(
                    RegistryOps.of(JsonOps.INSTANCE, registryManager),
                    enchant.value()
                ).getOrThrow()
            );
        }

        return enchantsJson;
    }
}
