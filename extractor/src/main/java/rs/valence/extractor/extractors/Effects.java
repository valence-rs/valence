package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import rs.valence.extractor.Main;
import rs.valence.extractor.ValenceUtils;

public class Effects implements Main.Extractor {

    public Effects() {}

    @Override
    public String fileName() {
        return "effects.json";
    }

    @Override
    public JsonElement extract() {
        var effectsJson = new JsonArray();

        for (var effect : Registries.STATUS_EFFECT) {
            var effectJson = new JsonObject();

            effectJson.addProperty(
                "id",
                Registries.STATUS_EFFECT.getRawId(effect)
            );
            effectJson.addProperty(
                "name",
                Registries.STATUS_EFFECT.getId(effect).getPath()
            );
            effectJson.addProperty(
                "translation_key",
                effect.getTranslationKey()
            );
            effectJson.addProperty("color", effect.getColor());
            effectJson.addProperty("instant", effect.isInstant());
            effectJson.addProperty("category", effect.getCategory().name());

            var attributeModifiersJson = new JsonArray();

            effect.forEachAttributeModifier(
                0,
                (attrRegistryEntry, modifier) -> {
                    var attributeModifierJson = new JsonObject();

                    var attr = attrRegistryEntry
                        .getKeyOrValue()
                        .map(k -> Registries.ATTRIBUTE.get(k), v -> v);
                    attributeModifierJson.addProperty(
                        "attribute_name",
                        attr
                            .getTranslationKey()
                            .replaceFirst("^attribute.name.", "")
                    );
                    attributeModifierJson.addProperty(
                        "operation",
                        modifier.operation().getId()
                    );
                    attributeModifierJson.addProperty(
                        "base_value",
                        modifier.value()
                    );

                    attributeModifiersJson.add(attributeModifierJson);
                }
            );

            if (attributeModifiersJson.size() > 0) {
                effectJson.add("attribute_modifiers", attributeModifiersJson);
            }

            effectsJson.add(effectJson);
        }

        return effectsJson;
    }
}
