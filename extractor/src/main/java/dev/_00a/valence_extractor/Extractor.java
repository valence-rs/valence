package dev._00a.valence_extractor;

import com.google.gson.*;
import net.fabricmc.api.ModInitializer;
import net.minecraft.block.BlockState;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityType;
import net.minecraft.entity.data.DataTracker;
import net.minecraft.entity.data.TrackedData;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.CatVariant;
import net.minecraft.entity.passive.FrogVariant;
import net.minecraft.item.ItemStack;
import net.minecraft.nbt.NbtCompound;
import net.minecraft.particle.ParticleEffect;
import net.minecraft.text.Text;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.Direction;
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
import java.util.*;

public class Extractor implements ModInitializer {
    public static final String MOD_ID = "valence_extractor";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);
    private Gson gson;
    private Path outputDirectory;

    private static JsonElement trackedDataToJson(Object data) {
        if (data instanceof BlockPos bp) {
            var json = new JsonObject();
            json.addProperty("x", bp.getX());
            json.addProperty("y", bp.getY());
            json.addProperty("z", bp.getZ());
            return json;
        } else if (data instanceof Boolean b) {
            return new JsonPrimitive(b);
        } else if (data instanceof Byte b) {
            return new JsonPrimitive(b);
        } else if (data instanceof CatVariant cv) {
            return new JsonPrimitive(Registry.CAT_VARIANT.getId(cv).getPath());
        } else if (data instanceof EntityPose ep) {
            return new JsonPrimitive(ep.toString());
        } else if (data instanceof Direction d) {
            return new JsonPrimitive(d.toString());
        } else if (data instanceof Float f) {
            return new JsonPrimitive(f);
        } else if (data instanceof FrogVariant fv) {
            return new JsonPrimitive(Registry.FROG_VARIANT.getId(fv).getPath());
        } else if (data instanceof Integer i) {
            return new JsonPrimitive(i);
        } else if (data instanceof ItemStack is) {
            // TODO
            return new JsonPrimitive(is.toString());
        } else if (data instanceof NbtCompound nbt) {
            // TODO: base64 binary representation or SNBT?
            return new JsonPrimitive(nbt.toString());
        } else if (data instanceof Optional<?> opt) {
            var inner = opt.orElse(null);
            if (inner == null) {
                return null;
            } else if (inner instanceof BlockPos) {
                return Extractor.trackedDataToJson(inner);
            } else if (inner instanceof BlockState bs) {
                // TODO: get raw block state ID.
                return new JsonPrimitive(bs.toString());
            } else if (inner instanceof GlobalPos gp) {
                var json = new JsonObject();
                json.addProperty("dimension", gp.getDimension().getValue().toString());

                var posJson = new JsonObject();
                posJson.addProperty("x", gp.getPos().getX());
                posJson.addProperty("y", gp.getPos().getY());
                posJson.addProperty("z", gp.getPos().getZ());

                json.add("position", posJson);
                return json;
            } else if (inner instanceof Text) {
                return Extractor.trackedDataToJson(inner);
            } else if (inner instanceof UUID uuid) {
                return new JsonPrimitive(uuid.toString());
            } else {
                throw new IllegalArgumentException("Unknown tracked optional type " + inner.getClass().getName());
            }
        } else if (data instanceof OptionalInt oi) {
            return oi.isPresent() ? new JsonPrimitive(oi.getAsInt()) : null;
        } else if (data instanceof RegistryEntry<?> re) {
            return new JsonPrimitive(re.getKey().map(k -> k.getValue().getPath()).orElse(""));
        } else if (data instanceof ParticleEffect pe) {
            return new JsonPrimitive(pe.asString());
        } else if (data instanceof EulerAngle ea) {
            var json = new JsonObject();
            json.addProperty("yaw", ea.getYaw());
            json.addProperty("pitch", ea.getPitch());
            json.addProperty("roll", ea.getRoll());
            return json;
        } else if (data instanceof String s) {
            return new JsonPrimitive(s);
        } else if (data instanceof Text t) {
            // TODO: return text as json element.
            return new JsonPrimitive(t.getString());
        } else if (data instanceof VillagerData vd) {
            var json = new JsonObject();
            json.addProperty("level", vd.getLevel());
            json.addProperty("type", vd.getType().toString());
            json.addProperty("profession", vd.getProfession().toString());
            return json;
        }

        throw new IllegalArgumentException("Unexpected tracked type " + data.getClass().getName());
    }

    @Override
    public void onInitialize() {
        LOGGER.info("Starting extractor...");

        try {
            outputDirectory = Files.createDirectories(Paths.get("valence_extractor_output"));
            gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().serializeNulls().create();

            extractBlocks();
            extractEntities();
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
                            fieldJson.addProperty("type_id", TrackedDataHandlerRegistry.getId(data.getType()));

                            var dataTracker = (DataTracker) dataTrackerField.get(entityInstance);
                            fieldJson.add("default_value", Extractor.trackedDataToJson(dataTracker.get(data)));

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

    private void writeJsonFile(String fileName, JsonElement element) throws IOException {
        var out = outputDirectory.resolve(fileName);
        var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
        gson.toJson(element, fileWriter);
        fileWriter.close();
        LOGGER.info("Wrote " + out.toAbsolutePath());
    }
}
