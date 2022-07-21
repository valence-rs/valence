package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;

import java.util.Locale;

public class EntityStatuses implements Main.Extractor {
    @Override
    public String fileName() {
        return "entity_statuses.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var statusesJson = new JsonObject();

        for (var field : net.minecraft.entity.EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                if (field.getName().equals("field_30030")) {
                    // TODO: temp
                    statusesJson.addProperty("stop_attack", code);
                } else {
                    statusesJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
                }
            }
        }

        return statusesJson;
    }
}
