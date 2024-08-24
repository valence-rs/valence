package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.component.DataComponentTypes;
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
            itemJson.addProperty("max_durability", item.getDefaultStack().getMaxDamage());
            itemJson.addProperty("enchantability", item.getEnchantability());
            itemJson.addProperty("fireproof", item.getComponents().contains(DataComponentTypes.FIRE_RESISTANT));

            if (item.getComponents().contains(DataComponentTypes.FOOD)) {
                var foodJson = new JsonObject();
                var foodComp = item.getComponents().get(DataComponentTypes.FOOD);

                foodJson.addProperty("hunger", foodComp.nutrition());
                foodJson.addProperty("saturation", foodComp.saturation());
                foodJson.addProperty("always_edible", foodComp.canAlwaysEat());
                foodJson.addProperty("eat_ticks", foodComp.getEatTicks());

                itemJson.add("food", foodJson);

                var effectsJson = new JsonArray();
                for (var effectEntry : foodComp.effects()) {
                    var effectJson = new JsonObject();

                    var effect = effectEntry.effect();
                    var chance = effectEntry.probability();

                    effectJson.addProperty("chance", chance);
                    effectJson.addProperty("translation_key", effect.getTranslationKey());
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
