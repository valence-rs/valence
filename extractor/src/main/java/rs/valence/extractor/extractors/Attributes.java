package rs.valence.extractor.extractors;

import java.io.DataOutput;

import com.google.gson.Gson;
import com.google.gson.JsonObject;

import net.minecraft.entity.attribute.ClampedEntityAttribute;
import net.minecraft.entity.attribute.EntityAttribute;
import net.minecraft.registry.Registries;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Attributes implements Main.Extractor {
    public Attributes() {
    }

    @Override
    public String fileName() {
        return "attributes.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson) throws Exception {
        var attributesJson = new JsonObject();

        for (EntityAttribute attribute : Registries.ATTRIBUTE) {
            var attributeJson = new JsonObject();

            attributeJson.addProperty("id", Registries.ATTRIBUTE.getRawId(attribute));
            attributeJson.addProperty("name", Registries.ATTRIBUTE.getId(attribute).getPath());
            attributeJson.addProperty("default_value", attribute.getDefaultValue());
            attributeJson.addProperty("translation_key", attribute.getTranslationKey());
            attributeJson.addProperty("tracked", attribute.isTracked());

            if (attribute instanceof ClampedEntityAttribute a) {
                attributeJson.addProperty("min_value", a.getMinValue());
                attributeJson.addProperty("max_value", a.getMaxValue());
            }

            attributesJson.add(Registries.ATTRIBUTE.getId(attribute).getPath(), attributeJson);
        }

        Main.writeJson(output, gson, attributesJson);
    }
}