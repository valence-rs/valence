package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
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

        for (var item : Registries.ITEM) {
            var itemJson = new JsonObject();
            itemJson.addProperty("id", Registries.ITEM.getRawId(item));
            itemJson.addProperty("name", Registries.ITEM.getId(item).getPath());
            itemJson.addProperty("translation_key", item.getTranslationKey());
            itemJson.addProperty("max_stack", item.getMaxCount());
            itemJson.addProperty("max_durability", item.getMaxDamage());
            itemJson.addProperty("enchantability", item.getEnchantability());
            itemJson.addProperty("fireproof", item.isFireproof());

            if (item.getFoodComponent() != null) {
                var foodJson = new JsonObject();
                var foodComp = item.getFoodComponent();

                foodJson.addProperty("hunger", foodComp.getHunger());
                foodJson.addProperty("saturation", foodComp.getSaturationModifier());
                foodJson.addProperty("always_edible", foodComp.isAlwaysEdible());
                foodJson.addProperty("meat", foodComp.isMeat());
                foodJson.addProperty("snack", foodComp.isSnack());

                itemJson.add("food", foodJson);

                var effectsJson = new JsonArray();
                for (var pair : foodComp.getStatusEffects()) {
                    var effectJson = new JsonObject();

                    var effect = pair.getFirst();
                    var chance = pair.getSecond();

                    effectJson.addProperty("chance", chance);
                    effectJson.addProperty("translation_key", effect.getEffectType().getTranslationKey());
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
