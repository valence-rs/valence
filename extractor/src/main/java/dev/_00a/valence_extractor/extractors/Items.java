package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
import net.minecraft.entity.EquipmentSlot;
import net.minecraft.util.registry.Registry;

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

        for (var item : Registry.ITEM) {
            var itemJson = new JsonObject();
            itemJson.addProperty("id", Registry.ITEM.getRawId(item));
            itemJson.addProperty("name", Registry.ITEM.getId(item).getPath());
            itemJson.addProperty("translation_key", item.getTranslationKey());
            itemJson.addProperty("max_stack", item.getMaxCount());



            if (item.isFood() && item.getFoodComponent() != null) {
                var hungerSaturationJson = new JsonObject();
                var foodComp = item.getFoodComponent();


                hungerSaturationJson.addProperty("hunger", foodComp.getHunger());
                hungerSaturationJson.addProperty("saturation", foodComp.getSaturationModifier());
                hungerSaturationJson.addProperty("always_edible", foodComp.isAlwaysEdible());
                hungerSaturationJson.addProperty("meat", foodComp.isMeat());
                hungerSaturationJson.addProperty("snack", foodComp.isSnack());

                var effectsJson = new JsonArray();

                // TODO: Implement when potions is implemented
                /*
                for (var effect : foodComp.getStatusEffects()) {
                    var effectJson = new JsonObject();

                    effectJson.addProperty("name", effect.getFirst().getEffectType().getName().getString());
                    effectJson.addProperty("translation_key", effect.getFirst().getTranslationKey());
                    effectJson.addProperty("duration", effect.getFirst().getDuration());
                    effectJson.addProperty("amplifier", effect.getFirst().getAmplifier());
                    effectJson.addProperty("permanent", effect.getFirst().isPermanent());
                    effectJson.addProperty("ambient", effect.getFirst().isAmbient());
                    effectJson.addProperty("show_icon", effect.getFirst().shouldShowIcon());
                    effectJson.addProperty("show_particles", effect.getFirst().shouldShowParticles());

                    effectsJson.add(effectJson);
                }
                */
                // To be removed when potions is implemented
                effectsJson.add(new JsonObject());

                hungerSaturationJson.add("effects", effectsJson);
                itemJson.add("food", hungerSaturationJson);
            }


            if (item.isDamageable()) {
                itemJson.addProperty("max_damage", item.getMaxDamage());
                itemJson.addProperty("enchantability", item.getEnchantability());
            }

            if(item.isFireproof()) {
                itemJson.addProperty("fireproof", true);
            }

            itemsJson.add(itemJson);
        }

        return itemsJson;
    }
}
