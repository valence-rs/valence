package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.registry.Registry;
import net.minecraft.world.EmptyBlockView;

public class Blocks implements Main.Extractor {
    public Blocks() {
    }

    @Override
    public String fileName() {
        return "blocks.json";
    }

    @Override
    public JsonElement extract() {
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
        return blocksJson;
    }
}
