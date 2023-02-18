package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.core.registries.BuiltInRegistries;
import rs.valence.extractor.Main;

public class Items implements Main.Extractor {
    public Items() {
    }

    @Override
    public String fileName() {
        return "items.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var itemsJson = new JsonArray();

        for (var item : BuiltInRegistries.ITEM) {
            var itemJson = new JsonObject();
            itemJson.addProperty("id", BuiltInRegistries.ITEM.getId(item));
            itemJson.addProperty("name", BuiltInRegistries.ITEM.getKey(item).getPath());
            itemJson.addProperty("translation_key", item.getDescriptionId());
            itemJson.addProperty("max_stack_size", item.getMaxStackSize());
            itemJson.addProperty("max_durability", item.getMaxDamage());
            itemJson.addProperty("enchantment_value", item.getEnchantmentValue());
            itemJson.addProperty("fireproof", item.isFireResistant());

            if (item.getFoodProperties() != null) {
                var foodJson = new JsonObject();
                var foodProps = item.getFoodProperties();

                foodJson.addProperty("nutrition", foodProps.getNutrition());
                foodJson.addProperty("saturation", foodProps.getSaturationModifier());
                foodJson.addProperty("always_edible", foodProps.canAlwaysEat());
                foodJson.addProperty("meat", foodProps.isMeat());
                foodJson.addProperty("fast_food", foodProps.isFastFood());

                itemJson.add("food", foodJson);

                var effectsJson = new JsonArray();
                for (var pair : foodProps.getEffects()) {
                    var effectJson = new JsonObject();

                    var effect = pair.getFirst();
                    var chance = pair.getSecond();

                    effectJson.addProperty("chance", chance);
                    effectJson.addProperty("translation_key", effect.getDescriptionId());
                    // TODO: more effect information.

                    effectsJson.add(effectJson);
                }

                foodJson.add("effects", effectsJson);
            }

            itemsJson.add(itemJson);
        }

        return itemsJson;
    }
}
