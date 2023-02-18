package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.level.EmptyBlockGetter;
import rs.valence.extractor.Main;

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

        var aabbs = new LinkedHashMap<AABB, Integer>();

        for (var block : BuiltInRegistries.BLOCK) {
            var blockJson = new JsonObject();

            blockJson.addProperty("id", BuiltInRegistries.BLOCK.getId(block));
            blockJson.addProperty("name", BuiltInRegistries.BLOCK.getKey(block).getPath());
            blockJson.addProperty("translation_key", block.getDescriptionId());
            blockJson.addProperty("item_id", BuiltInRegistries.ITEM.getId(block.asItem()));

            var propsJson = new JsonArray();
            for (var prop : block.getStateDefinition().getProperties()) {
                var propJson = new JsonObject();

                propJson.addProperty("name", prop.getName());

                var valuesJson = new JsonArray();
                for (var value : prop.getPossibleValues()) {
                    valuesJson.add(value.toString().toLowerCase(Locale.ROOT));
                }
                propJson.add("values", valuesJson);

                propsJson.add(propJson);
            }
            blockJson.add("properties", propsJson);

            var statesJson = new JsonArray();
            for (var state : block.getStateDefinition().getPossibleStates()) {
                var stateJson = new JsonObject();
                var id = stateIdCounter++;
                stateJson.addProperty("id", id);
                stateJson.addProperty("luminance", state.getLightEmission());
//                stateJson.addProperty("opaque", state.isOpaque());
                stateJson.addProperty("replaceable", state.getMaterial().isReplaceable());

                if (block.defaultBlockState().equals(state)) {
                    blockJson.addProperty("default_state_id", id);
                }

                var collisionShapeIdxsJson = new JsonArray();
                for (var aabb : state.getCollisionShape(EmptyBlockGetter.INSTANCE, BlockPos.ZERO).toAabbs()) {
                    var collisionAABB = new AABB(aabb.minX, aabb.minY, aabb.minZ, aabb.maxX, aabb.maxY, aabb.maxZ);

                    var idx = aabbs.putIfAbsent(collisionAABB, aabbs.size());
                    collisionShapeIdxsJson.add(Objects.requireNonNullElseGet(idx, () -> aabbs.size() - 1));
                }

                stateJson.add("collision_shapes", collisionShapeIdxsJson);

                statesJson.add(stateJson);
            }
            blockJson.add("states", statesJson);

            blocksJson.add(blockJson);
        }

        var shapesJson = new JsonArray();
        for (var shape : aabbs.keySet()) {
            var shapeJson = new JsonObject();
            shapeJson.addProperty("min_x", shape.minX);
            shapeJson.addProperty("min_y", shape.minY);
            shapeJson.addProperty("min_z", shape.minZ);
            shapeJson.addProperty("max_x", shape.maxX);
            shapeJson.addProperty("max_y", shape.maxY);
            shapeJson.addProperty("max_z", shape.maxZ);
            shapesJson.add(shapeJson);
        }

        topLevelJson.add("aabbs", shapesJson);
        topLevelJson.add("blocks", blocksJson);

        return topLevelJson;
    }

    private record AABB(double minX, double minY, double minZ, double maxX, double maxY, double maxZ) {
    }
}
