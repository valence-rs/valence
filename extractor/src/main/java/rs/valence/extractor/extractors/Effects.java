package rs.valence.extractor.extractors;

import java.io.DataOutput;
import java.io.IOException;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.entity.effect.StatusEffect;
import net.minecraft.registry.Registries;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Effects implements Main.Extractor {
    public Effects() {
    }

    @Override
    public String fileName() {
        return "effects.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson) throws IOException {
        var effectsJson = new JsonArray();

        for (var effect : Registries.STATUS_EFFECT) {
            var effectJson = new JsonObject();

            effectJson.addProperty("id", Registries.STATUS_EFFECT.getRawId(effect));
            effectJson.addProperty("name", Registries.STATUS_EFFECT.getId(effect).getPath());
            effectJson.addProperty("translation_key", effect.getTranslationKey());
            effectJson.addProperty("color", effect.getColor());
            effectJson.addProperty("instant", effect.isInstant());
            effectJson.addProperty("category", Main.toPascalCase(effect.getCategory().name()));

            var attributeModifiersJson = new JsonArray();

            for (var entry : effect.getAttributeModifiers().entrySet()) {
                var attributeModifierJson = new JsonObject();

                var attributeModidier = entry.getValue().createAttributeModifier(0);
                attributeModifierJson.addProperty("attribute", Registries.ATTRIBUTE.getRawId(entry.getKey()));
                attributeModifierJson.addProperty("operation", attributeModidier.getOperation().getId());
                attributeModifierJson.addProperty("base_value", attributeModidier.getValue());
                attributeModifierJson.addProperty("uuid", entry.getValue().getUuid().toString());

                attributeModifiersJson.add(attributeModifierJson);
            }

            if (attributeModifiersJson.size() > 0) {
                effectJson.add("attribute_modifiers", attributeModifiersJson);
            }

            effectsJson.add(effectJson);
        }

        Main.writeJson(output, gson, effectsJson);
    }
}
