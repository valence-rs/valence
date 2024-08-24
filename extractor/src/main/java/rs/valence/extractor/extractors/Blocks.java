package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.registry.Registries;
import net.minecraft.item.VerticallyAttachableBlockItem;
import net.minecraft.util.math.BlockPos;
import net.minecraft.world.EmptyBlockView;

import rs.valence.extractor.Main;
import rs.valence.extractor.mixin.ExposeWallBlock;

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
        final var topLevelJson = new JsonObject();

        final var blocksJson = new JsonArray();
        var stateIdCounter = 0;

        final var shapes = new LinkedHashMap<Shape, Integer>();

        for (final var block : Registries.BLOCK) {
            final var blockJson = new JsonObject();
            blockJson.addProperty("id", Registries.BLOCK.getRawId(block));
            blockJson.addProperty("name", Registries.BLOCK.getId(block).getPath());
            blockJson.addProperty("translation_key", block.getTranslationKey());
            blockJson.addProperty("item_id", Registries.ITEM.getRawId(block.asItem()));

            if (block.asItem() instanceof final VerticallyAttachableBlockItem wsbItem) {
                if (wsbItem.getBlock() == block) {
                    final var wallBlock = ((ExposeWallBlock) wsbItem).getWallBlock();
                    blockJson.addProperty("wall_variant_id", Registries.BLOCK.getRawId(wallBlock));
                }
            }

            final var propsJson = new JsonArray();
            for (final var prop : block.getStateManager().getProperties()) {
                final var propJson = new JsonObject();

                propJson.addProperty("name", prop.getName());

                final var valuesJson = new JsonArray();
                for (final var value : prop.getValues()) {
                    valuesJson.add(value.toString().toLowerCase(Locale.ROOT));
                }
                propJson.add("values", valuesJson);

                propsJson.add(propJson);
            }
            blockJson.add("properties", propsJson);

            final var statesJson = new JsonArray();
            for (final var state : block.getStateManager().getStates()) {
                final var stateJson = new JsonObject();
                final var id = stateIdCounter;
                stateIdCounter++;
                stateJson.addProperty("id", id);
                stateJson.addProperty("luminance", state.getLuminance());
                stateJson.addProperty("opaque", state.isOpaque());
                stateJson.addProperty("replaceable", state.isReplaceable());
                // TODO `blocksMovement` seems to be deprecated. How else can one get this property?
                stateJson.addProperty("blocks_motion", state.blocksMovement());

                if (block.getDefaultState().equals(state)) {
                    blockJson.addProperty("default_state_id", id);
                }

                final var collisionShapeIdxsJson = new JsonArray();
                for (final var box : state.getCollisionShape(EmptyBlockView.INSTANCE, BlockPos.ORIGIN).getBoundingBoxes()) {
                    final var collisionShape = new Shape(box.minX, box.minY, box.minZ, box.maxX, box.maxY, box.maxZ);

                    final var idx = shapes.putIfAbsent(collisionShape, shapes.size());
                    collisionShapeIdxsJson.add(Objects.requireNonNullElseGet(idx, () -> shapes.size() - 1));
                }

                stateJson.add("collision_shapes", collisionShapeIdxsJson);

                for (final var blockEntity : Registries.BLOCK_ENTITY_TYPE) {
                    if (blockEntity.supports(state)) {
                        stateJson.addProperty("block_entity_type", Registries.BLOCK_ENTITY_TYPE.getRawId(blockEntity));
                    }
                }

                statesJson.add(stateJson);
            }
            blockJson.add("states", statesJson);

            blocksJson.add(blockJson);
        }

        final var blockEntitiesJson = new JsonArray();
        for (final var blockEntity : Registries.BLOCK_ENTITY_TYPE) {
            final var blockEntityJson = new JsonObject();
            blockEntityJson.addProperty("id", Registries.BLOCK_ENTITY_TYPE.getRawId(blockEntity));
            blockEntityJson.addProperty("ident", Registries.BLOCK_ENTITY_TYPE.getId(blockEntity).toString());
            blockEntityJson.addProperty("name", Registries.BLOCK_ENTITY_TYPE.getId(blockEntity).getPath());

            blockEntitiesJson.add(blockEntityJson);
        }

        final var shapesJson = new JsonArray();
        for (final var shape : shapes.keySet()) {
            final var shapeJson = new JsonObject();
            shapeJson.addProperty("min_x", shape.minX);
            shapeJson.addProperty("min_y", shape.minY);
            shapeJson.addProperty("min_z", shape.minZ);
            shapeJson.addProperty("max_x", shape.maxX);
            shapeJson.addProperty("max_y", shape.maxY);
            shapeJson.addProperty("max_z", shape.maxZ);
            shapesJson.add(shapeJson);
        }

        topLevelJson.add("block_entity_types", blockEntitiesJson);
        topLevelJson.add("shapes", shapesJson);
        topLevelJson.add("blocks", blocksJson);

        return topLevelJson;
    }

    private record Shape(double minX, double minY, double minZ, double maxX, double maxY, double maxZ) {
    }
}
