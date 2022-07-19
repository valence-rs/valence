package dev._00a.valence_extractor;

import com.google.gson.*;
import net.fabricmc.api.ModInitializer;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.registry.Registry;
import net.minecraft.world.EmptyBlockView;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.FileWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

public class Extractor implements ModInitializer {
    public static final String MOD_ID = "valence_extractor";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);
    private Gson gson;
    private Path outputDirectory;

    @Override
    public void onInitialize() {
        LOGGER.info("Starting extractor...");

        try {
            outputDirectory = Files.createDirectories(Paths.get("valence_extractor_output"));
            gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();

            extractBlocks();
            extractEntities();
        } catch (Throwable e) {
            LOGGER.error("Extraction failed", e);
            System.exit(1);
        }

        LOGGER.info("Extractor finished successfully");
        System.exit(0);
    }

    void extractBlocks() throws IOException {
        var blocksJson = new JsonArray();
        var stateIdCounter = 0;

        for (var block : Registry.BLOCK) {
            var blockJson = new JsonObject();
//            blockJson.addProperty("id", Registry.BLOCK.getRawId(block));
            blockJson.addProperty("translation_key", block.getTranslationKey());
//            blockJson.addProperty("min_state_id", stateIdCounter);
//            blockJson.addProperty("max_state_id", stateIdCounter + block.getStateManager().getStates().size() - 1);

            var propsJson = new JsonArray();
            for (var prop : block.getStateManager().getProperties()) {
                var propJson = new JsonObject();

                propJson.addProperty("name", prop.getName());

                var valuesJson = new JsonArray();
                for (var value : prop.getValues()) {
                    valuesJson.add(value.toString());
                }
                propJson.add("values", valuesJson);

                propsJson.add(propJson);
            }
            blockJson.add("properties", propsJson);

            var statesJson = new JsonArray();
            for (var state : block.getStateManager().getStates()) {
                var stateJson = new JsonObject();
                var id = stateIdCounter++;
                stateJson.addProperty("id", id);
                stateJson.addProperty("luminance", state.getLuminance());
                stateJson.addProperty("opaque", state.isOpaque());

                if (block.getDefaultState().equals(state)) {
                    blockJson.addProperty("default_state_id", id);
                }

                var collisionShapesJson = new JsonArray();
                for (var box : state.getCollisionShape(EmptyBlockView.INSTANCE, BlockPos.ORIGIN).getBoundingBoxes()) {
                    var boxJson = new JsonObject();
                    boxJson.addProperty("min_x", box.minX);
                    boxJson.addProperty("min_y", box.minY);
                    boxJson.addProperty("min_z", box.minZ);
                    boxJson.addProperty("max_x", box.maxX);
                    boxJson.addProperty("max_y", box.maxY);
                    boxJson.addProperty("max_z", box.maxZ);
                    collisionShapesJson.add(boxJson);
                }
                stateJson.add("collision_shapes", collisionShapesJson);

                statesJson.add(stateJson);
            }
            blockJson.add("states", statesJson);

            blocksJson.add(blockJson);
        }

        writeJsonFile("blocks.json", blocksJson);
    }

    void extractEntities() throws IOException {
        var entitiesJson = new JsonArray();
        for (var entity : Registry.ENTITY_TYPE) {
            var entityJson = new JsonObject();
            entityJson.addProperty("translation_key", entity.getTranslationKey());

            entitiesJson.add(entityJson);
        }

        writeJsonFile("entities.json", entitiesJson);
    }

    void writeJsonFile(String fileName, JsonElement element) throws IOException {
        var out = outputDirectory.resolve(fileName);
        var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
        gson.toJson(element, fileWriter);
        fileWriter.close();
        LOGGER.info("Wrote " + out.toAbsolutePath());
    }
}
