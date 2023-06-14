package rs.valence.extractor.extractors;

import com.mojang.datafixers.util.Pair;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.entity.SpawnGroup;
import net.minecraft.util.Identifier;
import net.minecraft.util.collection.Weighted;
import net.minecraft.registry.Registries;
import net.minecraft.registry.Registry;
import net.minecraft.registry.RegistryKey;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryWrapper;
import net.minecraft.registry.BuiltinRegistries;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;
import net.minecraft.registry.tag.TagKey;
import net.minecraft.registry.CombinedDynamicRegistries;
import net.minecraft.registry.ServerDynamicRegistryType;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.registry.entry.RegistryEntryList;
import net.minecraft.registry.SerializableRegistries;

import java.util.Map;
import java.util.Map.Entry;
import java.util.HashMap;
import java.util.stream.Collectors;

public class Tags implements Main.Extractor {
    private CombinedDynamicRegistries<ServerDynamicRegistryType> dynamicRegistryManager;

    public Tags(MinecraftServer server) {
        this.dynamicRegistryManager = server.getCombinedDynamicRegistries();
    }

    @Override
    public String fileName() {
        return "tags.json";
    }

    @Override
    public JsonElement extract() {
        var tagsJson = new JsonArray();

        Map<RegistryKey<? extends Registry<?>>, Map<Identifier, JsonArray>> registryTags =
            SerializableRegistries.streamRegistryManagerEntries(this.dynamicRegistryManager)
            .map(registry -> Pair.of(registry.key(), serializeTags(registry.value())))
            .filter(pair -> !(pair.getSecond()).isEmpty())
            .collect(Collectors.toMap(Pair::getFirst, Pair::getSecond));

        for (var registry : registryTags.entrySet()) {
            var registryIdent = registry.getKey().getValue().toString();
            var tagGroupJson = new JsonObject();
            var tagGroupTagsJson = new JsonArray();

            for (var tag : registry.getValue().entrySet()) {
                var tagJson = new JsonObject();
                var ident = tag.getKey().toString();
                var raw_ids = tag.getValue();

                tagJson.addProperty("name", ident);
                tagJson.add("entries", raw_ids);
                tagGroupTagsJson.add(tagJson);
            }

            tagGroupJson.addProperty("registry", registryIdent.toString());
            tagGroupJson.add("tags", tagGroupTagsJson);
            tagsJson.add(tagGroupJson);
        }

        return tagsJson;
    }

    private static <T> Map<Identifier, JsonArray> serializeTags(Registry<T> registry) {
        HashMap<Identifier, JsonArray> map = new HashMap<Identifier, JsonArray>();
        registry.streamTagsAndEntries().forEach(pair -> {
            RegistryEntryList<T> registryEntryList = (RegistryEntryList<T>)pair.getSecond();
            JsonArray intList = new JsonArray(registryEntryList.size());
            for (RegistryEntry<T> registryEntry : registryEntryList) {
                if (registryEntry.getType() != RegistryEntry.Type.REFERENCE) {
                    throw new IllegalStateException("Can't serialize unregistered value " + registryEntry);
                }
                intList.add(registry.getRawId(registryEntry.value()));
            }
            map.put(((TagKey)pair.getFirst()).id(), intList);
        });
        return map;
    }
}
