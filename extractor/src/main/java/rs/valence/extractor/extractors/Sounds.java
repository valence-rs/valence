package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import rs.valence.extractor.Main;

public class Sounds implements Main.Extractor {

    public Sounds() {}

    @Override
    public String fileName() {
        return "sounds.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var itemsJson = new JsonArray();

        for (var sound : Registries.SOUND_EVENT) {
            var itemJson = new JsonObject();
            itemJson.addProperty("id", Registries.SOUND_EVENT.getRawId(sound));
            itemJson.addProperty(
                "name",
                Registries.SOUND_EVENT.getId(sound).getPath()
            );
            itemsJson.add(itemJson);
        }

        return itemsJson;
    }
}
