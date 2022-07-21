package dev._00a.valence_extractor;

import com.google.gson.*;
import net.fabricmc.api.ModInitializer;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.entity.EntityType;
import net.minecraft.entity.data.DataTracker;
import net.minecraft.entity.data.TrackedData;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.CatVariant;
import net.minecraft.entity.passive.FrogVariant;
import net.minecraft.item.ItemStack;
import net.minecraft.particle.ParticleEffect;
import net.minecraft.text.Text;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.EulerAngle;
import net.minecraft.util.math.GlobalPos;
import net.minecraft.util.registry.Registry;
import net.minecraft.util.registry.RegistryEntry;
import net.minecraft.village.VillagerData;
import net.minecraft.world.EmptyBlockView;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.FileWriter;
import java.io.IOException;
import java.lang.reflect.ParameterizedType;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.HashSet;
import java.util.Locale;
import java.util.Optional;
import java.util.OptionalInt;

public class Extractor implements ModInitializer {
    public static final String MOD_ID = "valence_extractor";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);
    private Gson gson;
    private Path outputDirectory;

    private static TD2JResult trackedDataToJson(TrackedData<?> data, DataTracker tracker) {
        final var handler = data.getType();
        final var val = tracker.get(data);

        if (handler == TrackedDataHandlerRegistry.BYTE) {
            return new TD2JResult("byte", new JsonPrimitive((Byte) val));
        } else if (handler == TrackedDataHandlerRegistry.INTEGER) {
            return new TD2JResult("integer", new JsonPrimitive((Integer) val));
        } else if (handler == TrackedDataHandlerRegistry.FLOAT) {
            return new TD2JResult("float", new JsonPrimitive((Float) val));
        } else if (handler == TrackedDataHandlerRegistry.STRING) {
            return new TD2JResult("string", new JsonPrimitive((String) val));
        } else if (handler == TrackedDataHandlerRegistry.TEXT_COMPONENT) {
            // TODO: return text as json element.
            return new TD2JResult("text_component", new JsonPrimitive(((Text) val).getString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_TEXT_COMPONENT) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(((Text) o).getString())).orElse(JsonNull.INSTANCE);
            return new TD2JResult("optional_text_component", res);
        } else if (handler == TrackedDataHandlerRegistry.ITEM_STACK) {
            // TODO
            return new TD2JResult("item_stack", new JsonPrimitive(((ItemStack) val).toString()));
        } else if (handler == TrackedDataHandlerRegistry.BOOLEAN) {
            return new TD2JResult("boolean", new JsonPrimitive((Boolean) val));
        } else if (handler == TrackedDataHandlerRegistry.ROTATION) {
            var json = new JsonObject();
            var ea = (EulerAngle) val;
            json.addProperty("pitch", ea.getPitch());
            json.addProperty("yaw", ea.getYaw());
            json.addProperty("roll", ea.getRoll());
            return new TD2JResult("rotation", json);
        } else if (handler == TrackedDataHandlerRegistry.BLOCK_POS) {
            var bp = (BlockPos) val;
            var json = new JsonObject();
            json.addProperty("x", bp.getX());
            json.addProperty("y", bp.getY());
            json.addProperty("z", bp.getZ());
            return new TD2JResult("block_pos", json);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_POS) {
            return new TD2JResult("optional_block_pos", ((Optional<?>) val).map(o -> {
                var bp = (BlockPos) o;
                var json = new JsonObject();
                json.addProperty("x", bp.getX());
                json.addProperty("y", bp.getY());
                json.addProperty("z", bp.getZ());
                return (JsonElement) json;
            }).orElse(JsonNull.INSTANCE));
        } else if (handler == TrackedDataHandlerRegistry.FACING) {
            return new TD2JResult("facing", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_UUID) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new TD2JResult("optional_uuid", res);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_STATE) {
            // TODO: get raw block state ID.
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new TD2JResult("optional_block_state", res);
        } else if (handler == TrackedDataHandlerRegistry.NBT_COMPOUND) {
            // TODO: base64 binary representation or SNBT?
            return new TD2JResult("nbt_compound", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.PARTICLE) {
            return new TD2JResult("particle", new JsonPrimitive(((ParticleEffect) val).asString()));
        } else if (handler == TrackedDataHandlerRegistry.VILLAGER_DATA) {
            var vd = (VillagerData) val;
            var json = new JsonObject();
            json.addProperty("type", vd.getType().toString());
            json.addProperty("profession", vd.getProfession().toString());
            json.addProperty("level", vd.getLevel());
            return new TD2JResult("villager_data", json);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_INT) {
            var opt = (OptionalInt) val;
            return new TD2JResult("optional_int", opt.isPresent() ? new JsonPrimitive(opt.getAsInt()) : JsonNull.INSTANCE);
        } else if (handler == TrackedDataHandlerRegistry.ENTITY_POSE) {
            return new TD2JResult("entity_pose", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.CAT_VARIANT) {
            return new TD2JResult("cat_variant", new JsonPrimitive(Registry.CAT_VARIANT.getId((CatVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.FROG_VARIANT) {
            return new TD2JResult("frog_variant", new JsonPrimitive(Registry.FROG_VARIANT.getId((FrogVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_GLOBAL_POS) {
            return new TD2JResult("optional_global_pos", ((Optional<?>) val).map(o -> {
                var gp = (GlobalPos) o;
                var json = new JsonObject();
                json.addProperty("dimension", gp.getDimension().getValue().toString());

                var posJson = new JsonObject();
                posJson.addProperty("x", gp.getPos().getX());
                posJson.addProperty("y", gp.getPos().getY());
                posJson.addProperty("z", gp.getPos().getZ());

                json.add("position", posJson);
                return (JsonElement) json;
            }).orElse(JsonNull.INSTANCE));
        } else if (handler == TrackedDataHandlerRegistry.PAINTING_VARIANT) {
            var variant = ((RegistryEntry<?>) val).getKey().map(k -> k.getValue().getPath()).orElse("");
            return new TD2JResult("painting_variant", new JsonPrimitive(variant));
        } else {
            throw new IllegalArgumentException("Unexpected tracked data type " + handler);
        }
    }

    @Override
    public void onInitialize() {
        LOGGER.info("Starting extractor...");

        try {
            outputDirectory = Files.createDirectories(Paths.get("valence_extractor_output"));
            gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().serializeNulls().create();

            extractBlocks();
            extractEntities();
            extractEntityStatuses();
        } catch (Throwable e) {
            LOGGER.error("Extraction failed", e);
            System.exit(1);
        }

        LOGGER.info("Extractor finished successfully.");
        System.exit(0);
    }

    private void extractBlocks() throws IOException {
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

    @SuppressWarnings("unchecked")
    private void extractEntities() throws IOException, IllegalAccessException, NoSuchFieldException {
        final var entitiesJson = new JsonArray();
        final var entityClasses = new HashSet<Class<? extends Entity>>();

        final var dummyWorld = DummyWorld.INSTANCE;

        for (var f : EntityType.class.getFields()) {
            if (f.getType().equals(EntityType.class)) {
                var entityType = (EntityType<?>) f.get(null);
                var entityClass = (Class<? extends Entity>) ((ParameterizedType) f.getGenericType()).getActualTypeArguments()[0];

                // While we can use the tracked data registry and reflection to get the tracked fields on entities, we won't know what their default values are because they are assigned in the entity's constructor.
                // To obtain this, we create a dummy world to spawn the entities into and then read the data tracker field from the base entity class.
                // We also handle player entities specially since they cannot be spawned with EntityType#create.
                final var entityInstance = entityType.equals(EntityType.PLAYER) ? DummyPlayerEntity.INSTANCE : entityType.create(dummyWorld);

                var dataTrackerField = Entity.class.getDeclaredField("dataTracker");
                dataTrackerField.setAccessible(true);

                while (entityClasses.add(entityClass)) {
                    var entityJson = new JsonObject();
                    entityJson.addProperty("class", entityClass.getSimpleName());
                    entityJson.add("translation_key", entityType != null ? new JsonPrimitive(entityType.getTranslationKey()) : null);

                    var fieldsJson = new JsonArray();
                    for (var entityField : entityClass.getDeclaredFields()) {
                        if (entityField.getType().equals(TrackedData.class)) {
                            entityField.setAccessible(true);

                            var data = (TrackedData<?>) entityField.get(null);

                            var fieldJson = new JsonObject();
                            fieldJson.addProperty("name", entityField.getName().toLowerCase(Locale.ROOT));
                            fieldJson.addProperty("index", data.getId());

                            var dataTracker = (DataTracker) dataTrackerField.get(entityInstance);
                            var res = Extractor.trackedDataToJson(data, dataTracker);
                            fieldJson.addProperty("type", res.type_name);
                            fieldJson.add("default_value", res.data);

                            fieldsJson.add(fieldJson);
                        }
                    }
                    entityJson.add("fields", fieldsJson);

                    var parent = entityClass.getSuperclass();
                    if (parent == null || !Entity.class.isAssignableFrom(parent)) {
                        entityJson.add("parent", null);
                        break;
                    }

                    entityJson.addProperty("parent", parent.getSimpleName());

                    entityClass = (Class<? extends Entity>) parent;
                    entityType = null;
                    entitiesJson.add(entityJson);
                }
            }
        }

        writeJsonFile("entities.json", entitiesJson);
    }

    private void extractEntityStatuses() throws IllegalAccessException, IOException {
        var statusesJson = new JsonObject();

        for (var field : EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                if (field.getName().equals("field_30030")) {
                    // TODO: temp
                    statusesJson.addProperty("stop_attack", code);
                } else {
                    statusesJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
                }
            }
        }

        writeJsonFile("entity_statuses.json", statusesJson);
    }

    private void writeJsonFile(String fileName, JsonElement element) throws IOException {
        var out = outputDirectory.resolve(fileName);
        var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
        gson.toJson(element, fileWriter);
        fileWriter.close();
        LOGGER.info("Wrote " + out.toAbsolutePath());
    }

    private record TD2JResult(String type_name, JsonElement data) {
    }
}
