package rs.valence.extractor.extractors;

import com.google.gson.*;

import it.unimi.dsi.fastutil.ints.Int2ObjectMap;
import net.minecraft.block.BlockState;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityType;
import net.minecraft.entity.LivingEntity;
import net.minecraft.entity.attribute.DefaultAttributeRegistry;
import net.minecraft.entity.attribute.EntityAttribute;
import net.minecraft.entity.attribute.EntityAttributeInstance;
import net.minecraft.entity.data.DataTracker;
import net.minecraft.entity.data.TrackedData;
import net.minecraft.entity.data.TrackedDataHandler;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.CatVariant;
import net.minecraft.entity.passive.FrogVariant;
import net.minecraft.entity.passive.SnifferEntity;
import net.minecraft.item.ItemStack;
import net.minecraft.particle.ParticleEffect;
import net.minecraft.registry.Registries;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.server.MinecraftServer;
import net.minecraft.text.Text;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.EulerAngle;
import net.minecraft.util.math.GlobalPos;
import net.minecraft.village.VillagerData;
import org.jetbrains.annotations.Nullable;
import org.joml.Quaternionf;
import org.joml.Vector3f;
import it.unimi.dsi.fastutil.ints.Int2ObjectMap;

import rs.valence.extractor.ClassComparator;
import rs.valence.extractor.DummyPlayerEntity;
import rs.valence.extractor.DummyWorld;
import rs.valence.extractor.Main;
import rs.valence.extractor.Main.Pair;

import java.io.DataOutput;
import java.io.IOException;
import java.lang.reflect.ParameterizedType;
import java.util.*;

public class Entities implements Main.Extractor {
    public Entities() {
    }

    @Override
    public String fileName() {
        return "entities.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson)
            throws IOException, IllegalAccessException, NoSuchFieldException {
        var entitiesJson = new JsonObject();

        for (var entityClass : Entities.ENTITY_CLASS_TO_TYPE_MAP.keySet()) {
            Entities.handleEntity(entityClass, entitiesJson);
        }

        Main.writeJson(output, gson, entitiesJson);
    }

    @SuppressWarnings("unchecked")
    private static void handleEntity(
            Class<? extends Entity> entityClass,
            JsonObject entitiesJson) throws IllegalAccessException, NoSuchFieldException {
        if (entitiesJson.get(entityClass.getSimpleName()) != null) {
            // Skip if we already handled this entity.
        }

        var entityJson = new JsonObject();

        var parentClass = entityClass.getSuperclass();
        if (parentClass != null && Entity.class.isAssignableFrom(parentClass)) {
            // If the entity has a parent class, handle that too.
            handleEntity((Class<? extends Entity>) parentClass, entitiesJson);

            entityJson.addProperty("parent", parentClass.getSimpleName());
        }

        var entityType = Entities.ENTITY_CLASS_TO_TYPE_MAP.get(entityClass);
        // Is this a concrete entity class?
        if (entityType != null) {
            // While we can use the tracked data registry and reflection to get the tracked
            // fields on entities, we won't know what their default values are because they
            // are assigned in the entity's constructor.
            // To obtain this, we create a dummy world to spawn the entities into and read
            // the data tracker field from the base entity class.
            // We also handle player entities specially since they cannot be spawned with
            // EntityType#create.
            var entityInstance = entityType.equals(EntityType.PLAYER)
                    ? DummyPlayerEntity.INSTANCE
                    : entityType.create(DummyWorld.INSTANCE);

            var dataTracker = entityInstance.getDataTracker();

            var dataTrackerEntriesField = DataTracker.class.getDeclaredField("entries");
            dataTrackerEntriesField.setAccessible(true);

            var dataTrackerEntries = (Int2ObjectMap<DataTracker.Entry<?>>) dataTrackerEntriesField.get(dataTracker);

            var defaultsMap = new TreeMap<Integer, JsonObject>();
            var defaults = (Int2ObjectMap<DataTracker.Entry<?>>) dataTrackerEntriesField.get(dataTracker);

            for (var entry : defaults.int2ObjectEntrySet()) {
                var fieldJson = new JsonObject();
                var trackedData = entry.getValue().getData();
                var data = Entities.trackedDataToJson(trackedData, dataTracker);
                int id = trackedData.getId();
                fieldJson.addProperty("index", id);
                fieldJson.add("value", data.right());
                fieldJson.addProperty("type", data.left());
                defaultsMap.put(id, fieldJson);
            }
            entityJson.add("defaults", Main.treeMapToJsonArray(defaultsMap));

            if (entityInstance instanceof LivingEntity livingEntity) {
                var type = (EntityType<? extends LivingEntity>) entityType;
                var defaultAttributes = DefaultAttributeRegistry.get(type);
                var attributesJson = new JsonArray();
                if (defaultAttributes != null) {
                    var instancesField = defaultAttributes.getClass().getDeclaredField("instances");
                    instancesField.setAccessible(true);
                    var instances = (Map<EntityAttribute, EntityAttributeInstance>) instancesField
                            .get(defaultAttributes);

                    for (var instance : instances.values()) {
                        var attribute = instance.getAttribute();

                        var attributeJson = new JsonObject();

                        attributeJson.addProperty("id", Registries.ATTRIBUTE.getRawId(attribute));
                        attributeJson.addProperty("name", Registries.ATTRIBUTE.getId(attribute).getPath());
                        attributeJson.addProperty("base_value", instance.getBaseValue());

                        attributesJson.add(attributeJson);
                    }
                }
                entityJson.add("attributes", attributesJson);
            }

            var bb = entityInstance.getBoundingBox();
            if (bb != null) {
                var boundingBoxJson = new JsonObject();

                boundingBoxJson.addProperty("size_x", bb.getLengthX());
                boundingBoxJson.addProperty("size_y", bb.getLengthY());
                boundingBoxJson.addProperty("size_z", bb.getLengthZ());

                entityJson.add("default_bounding_box", boundingBoxJson);
            }
        }

        var fieldsJson = new JsonArray();
        for (var entityField : entityClass.getDeclaredFields()) {
            if (entityField.getType().equals(TrackedData.class)) {
                entityField.setAccessible(true);

                var trackedData = (TrackedData<?>) entityField.get(null);

                var fieldJson = new JsonObject();
                var fieldName = entityField.getName().toLowerCase(Locale.ROOT);
                fieldJson.addProperty("name", fieldName);
                fieldJson.addProperty("index", trackedData.getId());

                // var data = Entities.trackedDataToJson(trackedData, dataTracker);
                fieldJson.addProperty("type", Entities.trackedDataHandlerName(trackedData.getType()));
                // fieldJson.add("default_value", data.right());

                fieldsJson.add(fieldJson);
            }
        }
        entityJson.add("fields", fieldsJson);

        entitiesJson.add(entityClass.getSimpleName(), entityJson);
    }

    private static final TreeMap<Class<? extends Entity>, EntityType<?>> ENTITY_CLASS_TO_TYPE_MAP = new TreeMap<>(
            new ClassComparator());
    static {
        for (var f : EntityType.class.getFields()) {
            if (f.getType().equals(EntityType.class)) {
                @SuppressWarnings("unchecked")
                var entityClass = (Class<? extends Entity>) ((ParameterizedType) f.getGenericType())
                        .getActualTypeArguments()[0];

                EntityType<?> entityType;

                try {
                    entityType = (EntityType<?>) f.get(null);
                } catch (IllegalAccessException e) {
                    throw new ExceptionInInitializerError(e);
                }

                ENTITY_CLASS_TO_TYPE_MAP.put(entityClass, entityType);
            }
        }
    }

    private static String trackedDataHandlerName(TrackedDataHandler<?> handler) {
        if (handler == TrackedDataHandlerRegistry.BYTE) {
            return "byte";
        } else if (handler == TrackedDataHandlerRegistry.INTEGER) {
            return "integer";
        } else if (handler == TrackedDataHandlerRegistry.LONG) {
            return "long";
        } else if (handler == TrackedDataHandlerRegistry.FLOAT) {
            return "float";
        } else if (handler == TrackedDataHandlerRegistry.STRING) {
            return "string";
        } else if (handler == TrackedDataHandlerRegistry.TEXT_COMPONENT) {
            return "text_component";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_TEXT_COMPONENT) {
            return "optional_text_component";
        } else if (handler == TrackedDataHandlerRegistry.ITEM_STACK) {
            return "item_stack";
        } else if (handler == TrackedDataHandlerRegistry.BOOLEAN) {
            return "boolean";
        } else if (handler == TrackedDataHandlerRegistry.ROTATION) {
            return "rotation";
        } else if (handler == TrackedDataHandlerRegistry.BLOCK_POS) {
            return "block_pos";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_POS) {
            return "optional_block_pos";
        } else if (handler == TrackedDataHandlerRegistry.FACING) {
            return "facing";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_UUID) {
            return "optional_uuid";
        } else if (handler == TrackedDataHandlerRegistry.BLOCK_STATE) {
            return "block_state";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_STATE) {
            return "optional_block_state";
        } else if (handler == TrackedDataHandlerRegistry.NBT_COMPOUND) {
            return "nbt_compound";
        } else if (handler == TrackedDataHandlerRegistry.PARTICLE) {
            return "particle";
        } else if (handler == TrackedDataHandlerRegistry.VILLAGER_DATA) {
            return "villager_data";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_INT) {
            return "optional_int";
        } else if (handler == TrackedDataHandlerRegistry.ENTITY_POSE) {
            return "entity_pose";
        } else if (handler == TrackedDataHandlerRegistry.CAT_VARIANT) {
            return "cat_variant";
        } else if (handler == TrackedDataHandlerRegistry.FROG_VARIANT) {
            return "frog_variant";
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_GLOBAL_POS) {
            return "optional_global_pos";
        } else if (handler == TrackedDataHandlerRegistry.PAINTING_VARIANT) {
            return "painting_variant";
        } else if (handler == TrackedDataHandlerRegistry.SNIFFER_STATE) {
            return "sniffer_state";
        } else if (handler == TrackedDataHandlerRegistry.VECTOR3F) {
            return "vector3f";
        } else if (handler == TrackedDataHandlerRegistry.QUATERNIONF) {
            return "quaternionf";
        } else {
            throw new IllegalArgumentException(
                    "Unknown tracked data handler of ID " + TrackedDataHandlerRegistry.getId(handler));
        }
    }

    private static Pair<String, JsonElement> trackedDataToJson(TrackedData<?> data, DataTracker tracker) {
        final var val = tracker.get(data);

        String name = trackedDataHandlerName(data.getType());
        JsonElement value = null;

        switch (name) {
            case "byte":
                value = new JsonPrimitive((Byte) val);
                break;
            case "integer":
                value = new JsonPrimitive((Integer) val);
                break;
            case "long":
                value = new JsonPrimitive((Long) val);
                break;
            case "float":
                value = new JsonPrimitive((Float) val);
                break;
            case "string":
                value = new JsonPrimitive((String) val);
                break;
            case "text_component":
                // TODO: return text as json element?
                value = new JsonPrimitive(((Text) val).getString());
                break;
            case "optional_text_component":
                value = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(((Text) o).getString()))
                        .orElse(JsonNull.INSTANCE);
                break;
            case "item_stack":
                // TODO
                value = new JsonPrimitive(((ItemStack) val).toString());
                break;
            case "boolean":
                value = new JsonPrimitive((Boolean) val);
                break;
            case "rotation": {
                var json = new JsonObject();
                var ea = (EulerAngle) val;
                json.addProperty("pitch", ea.getPitch());
                json.addProperty("yaw", ea.getYaw());
                json.addProperty("roll", ea.getRoll());
                value = json;
                break;
            }
            case "block_pos": {
                var bp = (BlockPos) val;
                var json = new JsonObject();
                json.addProperty("x", bp.getX());
                json.addProperty("y", bp.getY());
                json.addProperty("z", bp.getZ());
                value = json;
                break;
            }
            case "optional_block_pos":
                value = ((Optional<?>) val).map(o -> {
                    var bp = (BlockPos) o;
                    var json = new JsonObject();
                    json.addProperty("x", bp.getX());
                    json.addProperty("y", bp.getY());
                    json.addProperty("z", bp.getZ());
                    return (JsonElement) json;
                }).orElse(JsonNull.INSTANCE);
                break;
            case "facing":
                value = new JsonPrimitive(val.toString());
                break;
            case "optional_uuid":
                value = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString()))
                        .orElse(JsonNull.INSTANCE);
                break;
            case "block_state":
                value = new JsonPrimitive(((BlockState) val).toString());
                break;
            case "optional_block_state":
                value = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString()))
                        .orElse(JsonNull.INSTANCE);
                break;
            case "nbt_compound":
                // TODO: base64 binary representation or SNBT?
                value = new JsonPrimitive(val.toString());
                break;
            case "particle":
                value = new JsonPrimitive(Registries.PARTICLE_TYPE.getId(((ParticleEffect) val).getType()).getPath());
                break;
            case "villager_data": {
                var vd = (VillagerData) val;
                var json = new JsonObject();
                var type = Registries.VILLAGER_TYPE.getId(vd.getType()).getPath();
                var profession = Registries.VILLAGER_PROFESSION.getId(vd.getProfession()).getPath();
                json.addProperty("type", type);
                json.addProperty("profession", profession);
                json.addProperty("level", vd.getLevel());
                value = json;
                break;
            }
            case "optional_int": {
                var opt = (OptionalInt) val;
                value = opt.isPresent() ? new JsonPrimitive(opt.getAsInt()) : JsonNull.INSTANCE;
                break;
            }
            case "entity_pose":
                value = new JsonPrimitive(((EntityPose) val).name().toLowerCase(Locale.ROOT));
                break;
            case "cat_variant":
                value = new JsonPrimitive(Registries.CAT_VARIANT.getId((CatVariant) val).getPath());
                break;
            case "frog_variant":
                value = new JsonPrimitive(Registries.FROG_VARIANT.getId((FrogVariant) val).getPath());
                break;
            case "optional_global_pos":
                value = ((Optional<?>) val).map(o -> {
                    var gp = (GlobalPos) o;
                    var json = new JsonObject();
                    json.addProperty("dimension", gp.getDimension().getValue().toString());

                    var posJson = new JsonObject();
                    posJson.addProperty("x", gp.getPos().getX());
                    posJson.addProperty("y", gp.getPos().getY());
                    posJson.addProperty("z", gp.getPos().getZ());

                    json.add("position", posJson);
                    return (JsonElement) json;
                }).orElse(JsonNull.INSTANCE);
                break;
            case "painting_variant":
                value = new JsonPrimitive(
                        ((RegistryEntry<?>) val).getKey().map(k -> k.getValue().getPath()).orElse(""));
                break;
            case "sniffer_state":
                value = new JsonPrimitive(((SnifferEntity.State) val).name().toLowerCase(Locale.ROOT));
                break;
            case "vector3f": {
                var vec = (Vector3f) val;
                var json = new JsonObject();
                json.addProperty("x", vec.x);
                json.addProperty("y", vec.y);
                json.addProperty("z", vec.z);
                value = json;
                break;
            }
            case "quaternionf": {
                var quat = (Quaternionf) val;
                var json = new JsonObject();
                json.addProperty("x", quat.x);
                json.addProperty("y", quat.y);
                json.addProperty("z", quat.z);
                json.addProperty("w", quat.w);
                value = json;
                break;
            }
            default:
                throw new IllegalArgumentException("Unhandled tracked data handler of type \"" + name + "\"");
        }

        return new Pair<>(name, value);
    }
}
