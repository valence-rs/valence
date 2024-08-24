package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.datafixers.util.Pair;
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

import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;

public class Tags implements Main.Extractor {
    private final CombinedDynamicRegistries<ServerDynamicRegistryType> dynamicRegistryManager;

    public Tags(final MinecraftServer server) {
        dynamicRegistryManager = server.getCombinedDynamicRegistries();
    }

    @Override
    public String fileName() {
        return "tags.json";
    }

    @Override
    public JsonElement extract() {
        final var tagsJson = new JsonObject();

        var registryTags =
                SerializableRegistries.streamRegistryManagerEntries(dynamicRegistryManager)
                        .map(registry -> Pair.of(registry.key(), Tags.serializeTags(registry.value())))
                        .filter(pair -> !(pair.getSecond()).isEmpty())
                        .collect(Collectors.toMap(Pair::getFirst, Pair::getSecond, (l, r) -> r,
                                () -> new TreeMap<>(new RegistryKeyComparator())));

        for (final var registry : registryTags.entrySet()) {
            final var registryIdent = registry.getKey().getValue().toString();
            final var tagGroupTagsJson = new JsonObject();

            for (final var tag : registry.getValue().entrySet()) {
                final var ident = tag.getKey().toString();
                final var rawIds = tag.getValue();
                tagGroupTagsJson.add(ident, rawIds);
            }

            tagsJson.add(registryIdent, tagGroupTagsJson);
        }

        return tagsJson;
    }

    private static <T> Map<Identifier, JsonArray> serializeTags(final Registry<T> registry) {
        final TreeMap<Identifier, JsonArray> map = new TreeMap<>();
        registry.streamTagsAndEntries().forEach(pair -> {
            final RegistryEntryList<T> registryEntryList = pair.getSecond();
            final JsonArray intList = new JsonArray(registryEntryList.size());
            for (final RegistryEntry<T> registryEntry : registryEntryList) {
                if (RegistryEntry.Type.REFERENCE != registryEntry.getType()) {
                    throw new IllegalStateException("Can't serialize unregistered value " + registryEntry);
                }
                intList.add(registry.getRawId(registryEntry.value()));
            }
            map.put(pair.getFirst().id(), intList);
        });
        return map;
    }
}
