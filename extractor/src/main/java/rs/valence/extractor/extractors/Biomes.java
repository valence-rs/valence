package rs.valence.extractor.extractors;

import com.google.gson.*;
import net.minecraft.entity.SpawnGroup;
import net.minecraft.util.registry.BuiltinRegistries;
import rs.valence.extractor.Main;

import java.util.LinkedList;
import java.util.Optional;

public class Biomes implements Main.Extractor {
    public Biomes() {
    }

    @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
    private <T> JsonElement optional_to_json(Optional<T> var){
        if(var.isEmpty()){
            return JsonNull.INSTANCE;
        }else{
            var value = var.get();
            if(value instanceof Boolean){
                return new JsonPrimitive((Boolean) value);
            }else if(value instanceof Integer){
                return new JsonPrimitive((Integer) value);
            }else if(value instanceof Float){
                return new JsonPrimitive((Float) value);
            }else if(value instanceof Long){
                return new JsonPrimitive((Long) value);
            }else if(value instanceof Number){
                return new JsonPrimitive((Number) value);
            }else{
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
        var results = new LinkedList<JsonObject>();
        for (var biome_key : BuiltinRegistries.BIOME.getKeys()){
            var identifier = biome_key.getValue();
            var biome = BuiltinRegistries.BIOME.get(identifier);
            assert biome != null;

            var biomeJson = new JsonObject();

            var weatherJson = new JsonObject();
            weatherJson.addProperty("precipitation", biome.getPrecipitation().getName());
            weatherJson.addProperty("temperature", biome.getTemperature());
            weatherJson.addProperty("downfall", biome.getDownfall());

            var colorJson = new JsonObject();
            var biome_effects = biome.getEffects();
            colorJson.add("grass", optional_to_json(biome_effects.getGrassColor()));
            colorJson.addProperty("grass_modifier", biome_effects.getGrassColorModifier().getName());
            colorJson.add("foliage", optional_to_json(biome_effects.getFoliageColor()));
            colorJson.addProperty("fog", biome_effects.getFogColor());
            colorJson.addProperty("sky", biome_effects.getSkyColor());
            colorJson.addProperty("water_fog", biome_effects.getWaterFogColor());
            colorJson.addProperty("water", biome_effects.getWaterColor());

            var spawnSettingsJson = new JsonObject();
            var spawnSettings = biome.getSpawnSettings();
            spawnSettingsJson.addProperty("probability", spawnSettings.getCreatureSpawnProbability());

            var spawn_groups = new JsonArray();
            for (var spawn_group : SpawnGroup.values()){
                var group = new JsonObject();
                group.addProperty("name", spawn_group.getName());
                group.addProperty("capacity", spawn_group.getCapacity());
                group.addProperty("despawn_range_start", spawn_group.getDespawnStartRange());
                group.addProperty("despawn_range_immediate", spawn_group.getImmediateDespawnRange());
                group.addProperty("is_peaceful", spawn_group.isPeaceful());
                group.addProperty("is_rare", spawn_group.isRare());

                spawn_groups.add(group);
            }
            spawnSettingsJson.add("groups", spawn_groups);

            biomeJson.addProperty("name",identifier.toString());
            biomeJson.addProperty("id",BuiltinRegistries.BIOME.getRawId(biome));
            biomeJson.add("weather", weatherJson);
            biomeJson.add("color", colorJson);
            biomeJson.add("spawn_settings", spawnSettingsJson);

            results.add(biomeJson);
        }

        results.sort((one, two) -> {
            try{
                return one.get("id").getAsInt() - two.get("id").getAsInt();
            }catch (Exception e){
                throw new RuntimeException(e);
            }
        });

        var biomesJson = new JsonArray(results.size());
        results.forEach(biomesJson::add);
        return biomesJson;
    }
}
