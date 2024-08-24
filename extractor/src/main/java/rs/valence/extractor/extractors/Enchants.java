package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import net.minecraft.enchantment.Enchantment;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Enchants implements Main.Extractor {
    private final DynamicRegistryManager.Immutable registryManager;

    public Enchants(final MinecraftServer server) {
        registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "enchants.json";
    }

    @Override
    public JsonElement extract() {
        final var enchantsJson = new JsonObject();

        for (final var enchant : this.registryManager.get(RegistryKeys.ENCHANTMENT).streamEntries().toList()) {
            enchantsJson.add(enchant.getKey().orElseThrow().getValue().toString(), Enchantment.CODEC.encodeStart(RegistryOps.of(JsonOps.INSTANCE, this.registryManager), enchant.value()).getOrThrow());

        }

        return enchantsJson;
    }
}
