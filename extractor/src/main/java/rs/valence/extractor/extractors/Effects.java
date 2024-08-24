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
        final var effectsJson = new JsonArray();

        for (final var effect : Registries.STATUS_EFFECT) {
            final var effectJson = new JsonObject();

            effectJson.addProperty("id", Registries.STATUS_EFFECT.getRawId(effect));
            effectJson.addProperty("name", Registries.STATUS_EFFECT.getId(effect).getPath());
            effectJson.addProperty("translation_key", effect.getTranslationKey());
            effectJson.addProperty("color", effect.getColor());
            effectJson.addProperty("instant", effect.isInstant());
            effectJson.addProperty("category", ValenceUtils.toPascalCase(effect.getCategory().name()));

            final var attributeModifiersJson = new JsonArray();

            effect.forEachAttributeModifier(0, (attribute, modifier) -> {
                final var attributeModifierJson = new JsonObject();

                attributeModifierJson.addProperty("attribute", attribute.getIdAsString());
                attributeModifierJson.addProperty("operation", modifier.operation().getId());
                attributeModifierJson.addProperty("base_value", modifier.value());
                attributeModifierJson.addProperty("uuid", modifier.id().toTranslationKey());

                attributeModifiersJson.add(attributeModifierJson);
            });

            if (!attributeModifiersJson.isEmpty()) {
                effectJson.add("attribute_modifiers", attributeModifiersJson);
            }

            effectsJson.add(effectJson);
        }

        return effectsJson;
    }
}
