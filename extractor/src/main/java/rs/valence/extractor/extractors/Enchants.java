package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.core.registries.BuiltInRegistries;
import rs.valence.extractor.Main;

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

        for (var enchant : BuiltInRegistries.ENCHANTMENT) {
            var enchantJson = new JsonObject();

            enchantJson.addProperty("id", BuiltInRegistries.ENCHANTMENT.getId(enchant));
            enchantJson.addProperty("name", BuiltInRegistries.ENCHANTMENT.getKey(enchant).getPath());
            enchantJson.addProperty("translation_key", enchant.getDescriptionId());

            enchantJson.addProperty("min_level", enchant.getMinLevel());
            enchantJson.addProperty("max_level", enchant.getMaxLevel());
            enchantJson.addProperty("rarity_weight", enchant.getRarity().getWeight());
            enchantJson.addProperty("curse", enchant.isCurse());

            var enchantmentSources = new JsonObject();
            enchantmentSources.addProperty("treasure", enchant.isTreasureOnly());
//            enchantmentSources.addProperty("enchantment_table", enchant.isAvailableForEnchantedBookOffer());
//            // All enchants except for 'Soul speed' and 'Swift sneak' are available for random selection and are only obtainable from loot chests.
//            enchantmentSources.addProperty("random_selection", enchant.isAvailableForRandomSelection());

            enchantJson.add("sources", enchantmentSources);

            enchantsJson.add(enchantJson);
        }

        return enchantsJson;
    }
}
