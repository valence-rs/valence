package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import java.util.Optional;
import net.minecraft.component.ComponentMap;
import net.minecraft.component.DataComponentTypes;
import net.minecraft.component.type.EnchantableComponent;
import net.minecraft.item.Item;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.registry.tag.DamageTypeTags;
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

        for (var item : registryManager
            .getOrThrow(RegistryKeys.ITEM)
            .streamEntries()
            .toList()) {
            var itemJson = new JsonObject();

            itemJson.addProperty(
                "id",
                registryManager
                    .getOrThrow(RegistryKeys.ITEM)
                    .getRawId(item.value())
            );
            itemJson.addProperty(
                "name",
                item.getKey().orElseThrow().getValue().getPath()
            );
            Item realItem = item.value();
            itemJson.addProperty(
                "translation_key",
                realItem.getTranslationKey()
            );
            itemJson.addProperty("max_stack", realItem.getMaxCount());
            itemJson.addProperty(
                "max_durability",
                realItem.getDefaultStack().getMaxDamage()
            );
            itemJson.addProperty(
                "enchantability",
                Optional.ofNullable(
                    realItem.getComponents().get(DataComponentTypes.ENCHANTABLE)
                )
                    .map(EnchantableComponent::value)
                    .orElse(0)
            );
            itemJson.addProperty(
                "fireproof",
                Optional.ofNullable(
                    realItem
                        .getComponents()
                        .get(DataComponentTypes.DAMAGE_RESISTANT)
                )
                    .map(x -> x.types() == DamageTypeTags.IS_FIRE)
                    .orElse(false)
            );

            itemJson.add(
                "components",
                ComponentMap.CODEC.encodeStart(
                    RegistryOps.of(JsonOps.INSTANCE, registryManager),
                    realItem.getComponents()
                ).getOrThrow()
            );

            itemsJson.add(itemJson);
        }
        return itemsJson;
    }
}
