package rs.valence.extractor.extractors;

import com.google.gson.*;
import net.minecraft.entity.SpawnGroup;
import net.minecraft.util.collection.Weighted;
import net.minecraft.util.registry.BuiltinRegistries;
import net.minecraft.util.registry.Registry;
import rs.valence.extractor.Main;

import java.util.Optional;

public class Biomes implements Main.Extractor {
    public Biomes() {
    }

    @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
    private static <T> JsonElement optional_to_json(Optional<T> var) {
        if (var.isEmpty()) {
            return JsonNull.INSTANCE;
        } else {
            var value = var.get();
            if (value instanceof Boolean b) {
                return new JsonPrimitive(b);
            } else if (value instanceof Integer i) {
                return new JsonPrimitive(i);
            } else if (value instanceof Float f) {
                return new JsonPrimitive(f);
            } else if (value instanceof Long l) {
                return new JsonPrimitive(l);
            } else if (value instanceof Number n) {
                return new JsonPrimitive(n);
            } else {
                throw new UnsupportedOperationException("Could not convert " + value + " to primitive (" + value.getClass().toString() + ")");
            }
        }
    }

    @Override
    public String fileName() {
        return "biomes.json";
    }

    @Override
    public JsonElement extract() {
        var biomesJson = new JsonArray();

        for (var biome : BuiltinRegistries.BIOME) {
            var biomeIdent = BuiltinRegistries.BIOME.getId(biome);

            var climateJson = new JsonObject();
            climateJson.addProperty("precipitation", biome.getPrecipitation().getName());
            climateJson.addProperty("temperature", biome.getTemperature());
            climateJson.addProperty("downfall", biome.getDownfall());

            var colorJson = new JsonObject();
            var biomeEffects = biome.getEffects();
            colorJson.add("grass", optional_to_json(biomeEffects.getGrassColor()));
            colorJson.addProperty("grass_modifier", biomeEffects.getGrassColorModifier().getName());
            colorJson.add("foliage", optional_to_json(biomeEffects.getFoliageColor()));
            colorJson.addProperty("fog", biomeEffects.getFogColor());
            colorJson.addProperty("sky", biomeEffects.getSkyColor());
            colorJson.addProperty("water_fog", biomeEffects.getWaterFogColor());
            colorJson.addProperty("water", biomeEffects.getWaterColor());

            var spawnSettingsJson = new JsonObject();
            var spawnSettings = biome.getSpawnSettings();
            spawnSettingsJson.addProperty("probability", spawnSettings.getCreatureSpawnProbability());

            var spawnGroupsJson = new JsonObject();
            for (var spawnGroup : SpawnGroup.values()) {
                var spawnGroupJson = new JsonArray();
                for (var entry : spawnSettings.getSpawnEntries(spawnGroup).getEntries()) {
                    var groupEntryJson = new JsonObject();
                    groupEntryJson.addProperty("name", Registry.ENTITY_TYPE.getId(entry.type).getPath());
                    groupEntryJson.addProperty("min_group_size", entry.minGroupSize);
                    groupEntryJson.addProperty("max_group_size", entry.maxGroupSize);
                    groupEntryJson.addProperty("weight", ((Weighted) entry).getWeight().getValue());
                    spawnGroupJson.add(groupEntryJson);
                }
                spawnGroupsJson.add(spawnGroup.getName(), spawnGroupJson);
            }
            spawnSettingsJson.add("groups", spawnGroupsJson);

            var biomeJson = new JsonObject();
            biomeJson.addProperty("name", biomeIdent.getPath());
            biomeJson.addProperty("id", BuiltinRegistries.BIOME.getRawId(biome));
            biomeJson.add("climate", climateJson);
            biomeJson.add("color", colorJson);
            biomeJson.add("spawn_settings", spawnSettingsJson);

            biomesJson.add(biomeJson);
        }

        return biomesJson;
    }
}
