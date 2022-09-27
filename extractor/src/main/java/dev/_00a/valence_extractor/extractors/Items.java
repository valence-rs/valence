package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
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

            itemsJson.add(itemJson);
        }

        return itemsJson;
    }
}
