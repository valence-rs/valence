package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
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
//
        // TODO - For the moment I don't know how does this registers works
//        for (var enchant : Registries.ENCHANTMENT) {
//            var enchantJson = new JsonObject();
//
//            enchantJson.addProperty("id", Registries.ENCHANTMENT.getRawId(enchant));
//            enchantJson.addProperty("name", Registries.ENCHANTMENT.getId(enchant).getPath());
//            enchantJson.addProperty("translation_key", enchant.getTranslationKey());
//
//            enchantJson.addProperty("min_level", enchant.getMinLevel());
//            enchantJson.addProperty("max_level", enchant.getMaxLevel());
//            enchantJson.addProperty("rarity_weight", enchant.getRarity().getWeight());
//            enchantJson.addProperty("cursed", enchant.isCursed());
//
//            var enchantmentSources = new JsonObject();
//            enchantmentSources.addProperty("treasure", enchant.isTreasure());
//            enchantmentSources.addProperty("enchantment_table", enchant.isAvailableForEnchantedBookOffer());
//            // All enchants except for 'Soul speed' and 'Swift sneak' are available for random selection and are only obtainable from loot chests.
//            enchantmentSources.addProperty("random_selection", enchant.isAvailableForRandomSelection());
//
//            enchantJson.add("sources", enchantmentSources);
//
//            enchantsJson.add(enchantJson);
//        }

        return enchantsJson;
    }
}
