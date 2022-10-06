package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
import net.minecraft.util.registry.Registry;

public class Enchants implements Main.Extractor {
    public Enchants() {
    }

    @Override
    public String fileName() {
        return "enchants.json";
    }

    @Override
    public JsonElement extract() {
        var enchantsJson = new JsonArray();

        for (var enchant : Registry.ENCHANTMENT) {
            var enchantJson = new JsonObject();

            enchantJson.addProperty("id", Registry.ENCHANTMENT.getRawId(enchant));
            enchantJson.addProperty("name", Registry.ENCHANTMENT.getId(enchant).getPath());
            enchantJson.addProperty("translation_key", enchant.getTranslationKey());

            enchantJson.addProperty("min_level", enchant.getMinLevel());
            enchantJson.addProperty("max_level", enchant.getMaxLevel());
            enchantJson.addProperty("rarity_weight", enchant.getRarity().getWeight());
            enchantJson.addProperty("cursed", enchant.isCursed());

            var enchantmentSources = new JsonObject();
            enchantmentSources.addProperty("treasure", enchant.isTreasure());
            enchantmentSources.addProperty("enchantment_table", enchant.isAvailableForEnchantedBookOffer());
            // All enchants except for 'Soul speed' and 'Swift sneak' are available for random selection and are only obtainable from loot chests.
            enchantmentSources.addProperty("random_selection", enchant.isAvailableForRandomSelection());

            enchantJson.add("sources", enchantmentSources);

            enchantsJson.add(enchantJson);
        }

        return enchantsJson;
    }
}
