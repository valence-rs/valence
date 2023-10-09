package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import rs.valence.extractor.Main;

public class Effects implements Main.Extractor {
    public Effects() {
    }

    @Override
    public String fileName() {
        return "effects.json";
    }

    public static String toPascalCase(String str) {
        StringBuilder result = new StringBuilder();
        boolean capitalizeNext = true;

        for (char c : str.toCharArray()) {
            if (Character.isWhitespace(c) || c == '_') {
                capitalizeNext = true;
            } else if (capitalizeNext) {
                result.append(Character.toUpperCase(c));
                capitalizeNext = false;
            } else {
                result.append(Character.toLowerCase(c));
            }
        }

        return result.toString();
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
            effectJson.addProperty("category", toPascalCase(effect.getCategory().name()));
            
            effectsJson.add(effectJson);
        }

        return effectsJson;
    }
}
