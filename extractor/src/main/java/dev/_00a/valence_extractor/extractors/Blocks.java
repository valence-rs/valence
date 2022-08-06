package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.registry.Registry;
import net.minecraft.world.EmptyBlockView;

import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Objects;

public class Blocks implements Main.Extractor {
    public Blocks() {
    }

    @Override
    public String fileName() {
        return "blocks.json";
    }

    @Override
    public JsonElement extract() {
        var topLevelJson = new JsonObject();

        var blocksJson = new JsonArray();
        var stateIdCounter = 0;

        var shapes = new LinkedHashMap<Shape, Integer>();

        for (var block : Registry.BLOCK) {
            var blockJson = new JsonObject();
            blockJson.addProperty("id", Registry.BLOCK.getRawId(block));
            blockJson.addProperty("name", Registry.BLOCK.getId(block).getPath());
            blockJson.addProperty("translation_key", block.getTranslationKey());
//            blockJson.addProperty("min_state_id", stateIdCounter);
//            blockJson.addProperty("max_state_id", stateIdCounter + block.getStateManager().getStates().size() - 1);

            var propsJson = new JsonArray();
            for (var prop : block.getStateManager().getProperties()) {
                var propJson = new JsonObject();

                propJson.addProperty("name", prop.getName());

                var valuesJson = new JsonArray();
                for (var value : prop.getValues()) {
                    valuesJson.add(value.toString().toLowerCase(Locale.ROOT));
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

                var collisionShapeIdxsJson = new JsonArray();
                for (var box : state.getCollisionShape(EmptyBlockView.INSTANCE, BlockPos.ORIGIN).getBoundingBoxes()) {
                    var collisionShape = new Shape(box.minX, box.minY, box.minZ, box.maxX, box.maxY, box.maxZ);

                    var idx = shapes.putIfAbsent(collisionShape, shapes.size());
                    collisionShapeIdxsJson.add(Objects.requireNonNullElseGet(idx, () -> shapes.size() - 1));
                }

                stateJson.add("collision_shapes", collisionShapeIdxsJson);

                statesJson.add(stateJson);
            }
            blockJson.add("states", statesJson);

            blocksJson.add(blockJson);
        }

        var shapesJson = new JsonArray();
        for (var shape : shapes.keySet()) {
            var shapeJson = new JsonObject();
            shapeJson.addProperty("min_x", shape.minX);
            shapeJson.addProperty("min_y", shape.minY);
            shapeJson.addProperty("min_z", shape.minZ);
            shapeJson.addProperty("max_x", shape.maxX);
            shapeJson.addProperty("max_y", shape.maxY);
            shapeJson.addProperty("max_z", shape.maxZ);
            shapesJson.add(shapeJson);
        }

        topLevelJson.add("shapes", shapesJson);
        topLevelJson.add("blocks", blocksJson);

        return topLevelJson;
    }

    private record Shape(double minX, double minY, double minZ, double maxX, double maxY, double maxZ) {
    }
}
