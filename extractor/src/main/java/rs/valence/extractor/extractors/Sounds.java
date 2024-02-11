package rs.valence.extractor.extractors;

import java.io.DataOutput;
import java.io.IOException;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Sounds implements Main.Extractor {
    public Sounds() {
    }

    @Override
    public String fileName() {
        return "sounds.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson) throws IOException {
        var soundsJson = new JsonArray();

        for (var sound : Registries.SOUND_EVENT) {
            var soundJson = new JsonObject();
            soundJson.addProperty("id", Registries.SOUND_EVENT.getRawId(sound));
            soundJson.addProperty("name", Registries.SOUND_EVENT.getId(sound).getPath());
            soundsJson.add(soundJson);
        }

        Main.writeJson(output, gson, soundsJson);
    }
}
