package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import net.minecraft.component.ComponentMap;
import net.minecraft.component.DataComponentTypes;
import net.minecraft.component.type.FoodComponent;
import net.minecraft.enchantment.Enchantment;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Items implements Main.Extractor {
    private final DynamicRegistryManager.Immutable registryManager;

    public Items(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "items.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var itemsJson = new JsonArray();

        for (var item : registryManager.get(RegistryKeys.ITEM).streamEntries().toList()) {
            var itemJson = new JsonObject();

            itemJson.addProperty("id", registryManager.get(RegistryKeys.ITEM).getRawId(item.value()));
            itemJson.addProperty("name", item.getKey().orElseThrow().getValue().getPath());
            Item realItem = item.value();
            itemJson.addProperty("translation_key", realItem.getTranslationKey());
            itemJson.addProperty("max_stack", realItem.getMaxCount());
            itemJson.addProperty("max_durability", realItem.getDefaultStack().getMaxDamage());
            itemJson.addProperty("enchantability", realItem.getEnchantability());
            itemJson.addProperty("fireproof", realItem.getComponents().contains(DataComponentTypes.FIRE_RESISTANT));

            itemJson.add("components", ComponentMap.CODEC.encodeStart(RegistryOps.of(JsonOps.INSTANCE, registryManager), realItem.getComponents()).getOrThrow());

            itemsJson.add(itemJson);
        }
        return itemsJson;
    }
}
