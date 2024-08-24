package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.util.Language;
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
        final JsonArray translationsJson = new JsonArray();

        final Map<String, String> translations = TranslationKeys.extractTranslations();
        for (final var translation : translations.entrySet()) {
            final String translationKey = translation.getKey();
            final String translationValue = translation.getValue();

            final var translationJson = new JsonObject();
            translationJson.addProperty("key", translationKey);
            translationJson.addProperty("english_translation", translationValue);

            translationsJson.add(translationJson);
        }

        return translationsJson;
    }

    @SuppressWarnings("unchecked")
    private static Map<String, String> extractTranslations() {
        final Language language = Language.getInstance();

        final Class<? extends Language> anonymousClass = language.getClass();
        for (final Field field : anonymousClass.getDeclaredFields()) {
            try {
                final Object fieldValue = field.get(language);
                if (fieldValue instanceof Map<?, ?>) {
                    return (Map<String, String>) fieldValue;
                }
            } catch (final IllegalAccessException e) {
                throw new RuntimeException("Failed reflection on field '" + field + "' on class '" + anonymousClass + "'", e);
            }
        }

        throw new RuntimeException("Did not find anonymous map under 'net.minecraft.util.Language.create()'");
    }
}
