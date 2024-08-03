package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

public class Paintings implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;

    public Paintings(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "paintings.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var dataDrivenMiscJson = new JsonObject();

        // TODO - For the moment I don't know how does this registers works
        var paintingRegistryWrapper = registryManager.getWrapperOrThrow(RegistryKeys.PAINTING_VARIANT);

        var paintingVariantJson = new JsonObject();
        paintingRegistryWrapper.streamKeys().forEach(variant -> {
            //Main.LOGGER.info("Painting variant: {}", variant);
//            var variantJson = new JsonObject();
//            variantJson.addProperty("id", Registries.PAINTING_MOTIVE.getRawId(variant));
//            variantJson.addProperty("width", variant.getWidth());
//            variantJson.addProperty("height", variant.getHeight());
//            paintingVariantJson.add(Registries.PAINTING_VARIANT.getId(variant).getPath(), variantJson);
        });
//        for (var variant : paintingRegistryWrapper.streamKeys()) {
//            var variantJson = new JsonObject();
//            variantJson.addProperty("id", Registries.PAINTING_MOTIVE.getRawId(variant));
//            variantJson.addProperty("width", variant.getWidth());
//            variantJson.addProperty("height", variant.getHeight());
//            paintingVariantJson.add(Registries.PAINTING_VARIANT.getId(variant).getPath(), variantJson);
//        }
        dataDrivenMiscJson.add("painting_variant", paintingVariantJson);

        return dataDrivenMiscJson;
    }
}
