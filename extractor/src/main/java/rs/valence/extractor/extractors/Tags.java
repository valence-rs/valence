package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.datafixers.util.Pair;
import net.minecraft.registry.*;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.registry.entry.RegistryEntryList;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.Identifier;
import rs.valence.extractor.Main;

import java.util.HashMap;
import java.util.Map;
import java.util.stream.Collectors;

public class Tags implements Main.Extractor {
    private final CombinedDynamicRegistries<ServerDynamicRegistryType> dynamicRegistryManager;

    public Tags(MinecraftServer server) {
        this.dynamicRegistryManager = server.getCombinedDynamicRegistries();
    }

    @Override
    public String fileName() {
        return "tags.json";
    }

    @Override
    public JsonElement extract() {
        var tagsJson = new JsonObject();

        Map<RegistryKey<? extends Registry<?>>, Map<Identifier, JsonArray>> registryTags =
            SerializableRegistries.streamRegistryManagerEntries(this.dynamicRegistryManager)
            .map(registry -> Pair.of(registry.key(), serializeTags(registry.value())))
            .filter(pair -> !(pair.getSecond()).isEmpty())
            .collect(Collectors.toMap(Pair::getFirst, Pair::getSecond));

        for (var registry : registryTags.entrySet()) {
            var registryIdent = registry.getKey().getValue().toString();
            var tagGroupTagsJson = new JsonObject();

            for (var tag : registry.getValue().entrySet()) {
                var ident = tag.getKey().toString();
                var rawIds = tag.getValue();
                tagGroupTagsJson.add(ident, rawIds);
            }

            tagsJson.add(registryIdent, tagGroupTagsJson);
        }

        return tagsJson;
    }

    private static <T> Map<Identifier, JsonArray> serializeTags(Registry<T> registry) {
        HashMap<Identifier, JsonArray> map = new HashMap<>();
        registry.streamTagsAndEntries().forEach(pair -> {
            RegistryEntryList<T> registryEntryList = pair.getSecond();
            JsonArray intList = new JsonArray(registryEntryList.size());
            for (RegistryEntry<T> registryEntry : registryEntryList) {
                if (registryEntry.getType() != RegistryEntry.Type.REFERENCE) {
                    throw new IllegalStateException("Can't serialize unregistered value " + registryEntry);
                }
                intList.add(registry.getRawId(registryEntry.value()));
            }
            map.put(pair.getFirst().id(), intList);
        });
        return map;
    }
}
