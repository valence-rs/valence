package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.locale.Language;
import rs.valence.extractor.Main;

import java.lang.reflect.Field;
import java.util.Map;

public class TranslationKeys implements Main.Extractor {

    @Override
    public String fileName() {
        return "translation_keys.json";
    }

    @Override
    public JsonElement extract() {
        JsonArray translationsJson = new JsonArray();

        Map<String, String> translations = extractTranslations();
        for (var translation : translations.entrySet()) {
            String translationKey = translation.getKey();
            String translationValue = translation.getValue();

            var translationJson = new JsonObject();
            translationJson.addProperty("key", translationKey);
            translationJson.addProperty("english_translation", translationValue);

            translationsJson.add(translationJson);
        }

        return translationsJson;
    }

    @SuppressWarnings("unchecked")
    private static Map<String, String> extractTranslations() {
        Language language = Language.getInstance();

        Class<? extends Language> anonymousClass = language.getClass();
        for (Field field : anonymousClass.getDeclaredFields()) {
            try {
                Object fieldValue = field.get(language);
                if (fieldValue instanceof Map<?, ?>) {
                    return (Map<String, String>) fieldValue;
                }
            } catch (IllegalAccessException e) {
                throw new RuntimeException("Failed reflection on field '" + field + "' on class '" + anonymousClass + "'", e);
            }
        }

        throw new RuntimeException("Did not find anonymous map under 'net.minecraft.util.Language.create()'");
    }
}
