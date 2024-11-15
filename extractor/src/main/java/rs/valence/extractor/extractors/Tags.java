package rs.valence.extractor.extractors;

import com.google.common.collect.Lists;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.datafixers.util.Pair;

import java.util.ArrayList;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;
import net.minecraft.registry.CombinedDynamicRegistries;
import net.minecraft.registry.Registry;
import net.minecraft.registry.SerializableRegistries;
import net.minecraft.registry.ServerDynamicRegistryType;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.registry.entry.RegistryEntryList;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.Identifier;
import rs.valence.extractor.Main;
import rs.valence.extractor.RegistryKeyComparator;

public class Tags implements Main.Extractor {

    private final CombinedDynamicRegistries<
        ServerDynamicRegistryType
    > dynamicRegistryManager;

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

        final var registryTags =
            SerializableRegistries.streamRegistryManagerEntries(
                this.dynamicRegistryManager
            )
                .map(registry ->
                    Pair.of(registry.key(), serializeTags(registry.value()))
                )
                .filter(pair -> !(pair.getSecond()).isEmpty())
                .collect(
                    Collectors.toMap(
                        Pair::getFirst,
                        Pair::getSecond,
                        (l, r) -> r,
                        () -> new TreeMap<>(new RegistryKeyComparator())
                    )
                );

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

    private static <T> Map<Identifier, JsonArray> serializeTags(
        Registry<T> registry
    ) {
        TreeMap<Identifier, JsonArray> map = new TreeMap<>();
        registry
                .streamTags()
                .map(key -> Pair.of(key, registry.iterateEntries(key.getTag())))
            .forEach(pair -> {
                var registryEntryList = Lists.newArrayList(pair.getSecond());
                JsonArray intList = new JsonArray(registryEntryList.size());
                for (RegistryEntry<T> registryEntry : registryEntryList) {
                    if (
                        RegistryEntry.Type.REFERENCE != registryEntry.getType()
                    ) {
                        throw new IllegalStateException(
                            "Can't serialize unregistered value " +
                            registryEntry
                        );
                    }
                    intList.add(registry.getRawId(registryEntry.value()));
                }
                map.put(pair.getFirst().getTag().id(), intList);
            });
        return map;
    }
}
