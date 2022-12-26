package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.entity.SpawnGroup;
import net.minecraft.util.Identifier;
import net.minecraft.util.collection.Weighted;
import net.minecraft.util.registry.BuiltinRegistries;
import net.minecraft.util.registry.Registry;
import net.minecraft.world.biome.BiomeParticleConfig;
import rs.valence.extractor.Main;

import java.lang.reflect.Field;

public class Biomes implements Main.Extractor {
    public Biomes() {
    }

    @Override
    public String fileName() {
        return "biomes.json";
    }

    @Override
    public JsonElement extract() {
        // The biome particle probability field is private.
        // We have to resort to reflection, unfortunately.
        Field particleConfigProbabilityField;
        try {
            particleConfigProbabilityField = BiomeParticleConfig.class.getDeclaredField("probability");
            particleConfigProbabilityField.setAccessible(true);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        var biomesJson = new JsonArray();

        for (var biome : BuiltinRegistries.BIOME) {
            var biomeIdent = BuiltinRegistries.BIOME.getId(biome);
            assert biomeIdent != null;

            var biomeJson = new JsonObject();
            biomeJson.addProperty("precipitation", biome.getPrecipitation().getName());
            biomeJson.addProperty("temperature", biome.getTemperature());
            biomeJson.addProperty("downfall", biome.getDownfall());

            var effectJson = new JsonObject();
            var biomeEffects = biome.getEffects();

            effectJson.addProperty("sky_color", biomeEffects.getSkyColor());
            effectJson.addProperty("water_fog_color", biomeEffects.getWaterFogColor());
            effectJson.addProperty("fog_color", biomeEffects.getFogColor());
            effectJson.addProperty("water_color", biomeEffects.getWaterColor());
            biomeEffects.getFoliageColor().ifPresent(color -> effectJson.addProperty("foliage_color", color));
            biomeEffects.getGrassColor().ifPresent(color -> effectJson.addProperty("grass_color", color));
            effectJson.addProperty("grass_color_modifier", biomeEffects.getGrassColorModifier().getName());
            biomeEffects.getMusic().ifPresent(biome_music -> {
                var music = new JsonObject();
                music.addProperty("replace_current_music", biome_music.shouldReplaceCurrentMusic());
                music.addProperty("sound", biome_music.getSound().getId().getPath());
                music.addProperty("max_delay", biome_music.getMaxDelay());
                music.addProperty("min_delay", biome_music.getMinDelay());
                effectJson.add("music", music);
            });

            biomeEffects.getLoopSound().ifPresent(soundEvent -> effectJson.addProperty("ambient_sound", soundEvent.getId().getPath()));
            biomeEffects.getAdditionsSound().ifPresent(soundEvent -> {
                var sound = new JsonObject();
                sound.addProperty("sound", soundEvent.getSound().getId().getPath());
                sound.addProperty("tick_chance", soundEvent.getChance());
                effectJson.add("additions_sound", sound);
            });
            biomeEffects.getMoodSound().ifPresent(soundEvent -> {
                var sound = new JsonObject();
                sound.addProperty("sound", soundEvent.getSound().getId().getPath());
                sound.addProperty("tick_delay", soundEvent.getCultivationTicks());
                sound.addProperty("offset", soundEvent.getExtraDistance());
                sound.addProperty("block_search_extent", soundEvent.getSpawnRange());

                effectJson.add("mood_sound", sound);
            });

            biome.getParticleConfig().ifPresent(biomeParticleConfig -> {
                try {
                    var particleConfig = new JsonObject();
                    // We must first convert it into an identifier, because asString() returns a resource identifier as string.
                    Identifier id = new Identifier(biomeParticleConfig.getParticle().asString());
                    particleConfig.addProperty("kind", id.getPath());
                    particleConfig.addProperty("probability" ,particleConfigProbabilityField.getFloat(biomeParticleConfig));
                    biomeJson.add("particle", particleConfig);
                } catch (IllegalAccessException e) {
                    throw new RuntimeException(e);
                }
            });

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

            biomeJson.add("effects", effectJson);
            biomeJson.add("spawn_settings", spawnSettingsJson);

            var entryJson = new JsonObject();
            entryJson.addProperty("name", biomeIdent.getPath());
            entryJson.addProperty("id", BuiltinRegistries.BIOME.getRawId(biome));
            entryJson.add("element", biomeJson);
            biomesJson.add(entryJson);
        }

        return biomesJson;
    }
}
