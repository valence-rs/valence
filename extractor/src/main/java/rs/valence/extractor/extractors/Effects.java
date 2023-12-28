package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import rs.valence.extractor.Main;
import rs.valence.extractor.ValenceUtils;

public class Effects implements Main.Extractor {
    public Effects() {
    }

    @Override
    public String fileName() {
        return "effects.json";
    }

    @Override
    public JsonElement extract() {
        var effectsJson = new JsonArray();

        for (var effect : Registries.STATUS_EFFECT) {
            var effectJson = new JsonObject();

            effectJson.addProperty("id", Registries.STATUS_EFFECT.getRawId(effect));
            effectJson.addProperty("name", Registries.STATUS_EFFECT.getId(effect).getPath());
            effectJson.addProperty("translation_key", effect.getTranslationKey());
            effectJson.addProperty("color", effect.getColor());
            effectJson.addProperty("instant", effect.isInstant());
            effectJson.addProperty("category", ValenceUtils.toPascalCase(effect.getCategory().name()));

            var attributeModifiersJson = new JsonArray();

            for (var entry : effect.getAttributeModifiers().entrySet()) {
                var attributeModifierJson = new JsonObject();

                attributeModifierJson.addProperty("attribute", Registries.ATTRIBUTE.getRawId(entry.getKey()));
                attributeModifierJson.addProperty("operation", entry.getValue().getOperation().getId());
                attributeModifierJson.addProperty("value", entry.getValue().getValue());
                attributeModifierJson.addProperty("uuid", entry.getValue().getId().toString());

                attributeModifiersJson.add(attributeModifierJson);
            }

            if (attributeModifiersJson.size() > 0) {
                effectJson.add("attribute_modifiers", attributeModifiersJson);
            }

            effectsJson.add(effectJson);
        }

        return effectsJson;
    }
}
