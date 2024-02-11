package rs.valence.extractor.extractors;

import com.google.gson.*;

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

    private static Pair<String, JsonElement> trackedDataToJson(TrackedData<?> data, DataTracker tracker) {
        final var handler = data.getType();
        final var val = tracker.get(data);

        if (handler == TrackedDataHandlerRegistry.BYTE) {
            return new Pair<>("byte", new JsonPrimitive((Byte) val));
        } else if (handler == TrackedDataHandlerRegistry.INTEGER) {
            return new Pair<>("integer", new JsonPrimitive((Integer) val));
        } else if (handler == TrackedDataHandlerRegistry.LONG) {
            return new Pair<>("long", new JsonPrimitive((Long) val));
        } else if (handler == TrackedDataHandlerRegistry.FLOAT) {
            return new Pair<>("float", new JsonPrimitive((Float) val));
        } else if (handler == TrackedDataHandlerRegistry.STRING) {
            return new Pair<>("string", new JsonPrimitive((String) val));
        } else if (handler == TrackedDataHandlerRegistry.TEXT_COMPONENT) {
            // TODO: return text as json element.
            return new Pair<>("text_component", new JsonPrimitive(((Text) val).getString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_TEXT_COMPONENT) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(((Text) o).getString()))
                    .orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_text_component", res);
        } else if (handler == TrackedDataHandlerRegistry.ITEM_STACK) {
            // TODO
            return new Pair<>("item_stack", new JsonPrimitive(((ItemStack) val).toString()));
        } else if (handler == TrackedDataHandlerRegistry.BOOLEAN) {
            return new Pair<>("boolean", new JsonPrimitive((Boolean) val));
        } else if (handler == TrackedDataHandlerRegistry.ROTATION) {
            var json = new JsonObject();
            var ea = (EulerAngle) val;
            json.addProperty("pitch", ea.getPitch());
            json.addProperty("yaw", ea.getYaw());
            json.addProperty("roll", ea.getRoll());
            return new Pair<>("rotation", json);
        } else if (handler == TrackedDataHandlerRegistry.BLOCK_POS) {
            var bp = (BlockPos) val;
            var json = new JsonObject();
            json.addProperty("x", bp.getX());
            json.addProperty("y", bp.getY());
            json.addProperty("z", bp.getZ());
            return new Pair<>("block_pos", json);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_POS) {
            return new Pair<>("optional_block_pos", ((Optional<?>) val).map(o -> {
                var bp = (BlockPos) o;
                var json = new JsonObject();
                json.addProperty("x", bp.getX());
                json.addProperty("y", bp.getY());
                json.addProperty("z", bp.getZ());
                return (JsonElement) json;
            }).orElse(JsonNull.INSTANCE));
        } else if (handler == TrackedDataHandlerRegistry.FACING) {
            return new Pair<>("facing", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_UUID) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString()))
                    .orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_uuid", res);
        } else if (handler == TrackedDataHandlerRegistry.BLOCK_STATE) {
            // TODO: get raw block state ID.
            var state = (BlockState) val;
            return new Pair<>("block_state", new JsonPrimitive(state.toString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_STATE) {
            // TODO: get raw block state ID.
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString()))
                    .orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_block_state", res);
        } else if (handler == TrackedDataHandlerRegistry.NBT_COMPOUND) {
            // TODO: base64 binary representation or SNBT?
            return new Pair<>("nbt_compound", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.PARTICLE) {
            var id = Registries.PARTICLE_TYPE.getId(((ParticleEffect) val).getType());
            return new Pair<>("particle", new JsonPrimitive(id.getPath()));
        } else if (handler == TrackedDataHandlerRegistry.VILLAGER_DATA) {
            var vd = (VillagerData) val;
            var json = new JsonObject();
            var type = Registries.VILLAGER_TYPE.getId(vd.getType()).getPath();
            var profession = Registries.VILLAGER_PROFESSION.getId(vd.getProfession()).getPath();
            json.addProperty("type", type);
            json.addProperty("profession", profession);
            json.addProperty("level", vd.getLevel());
            return new Pair<>("villager_data", json);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_INT) {
            var opt = (OptionalInt) val;
            return new Pair<>("optional_int", opt.isPresent() ? new JsonPrimitive(opt.getAsInt()) : JsonNull.INSTANCE);
        } else if (handler == TrackedDataHandlerRegistry.ENTITY_POSE) {
            return new Pair<>("entity_pose", new JsonPrimitive(((EntityPose) val).name().toLowerCase(Locale.ROOT)));
        } else if (handler == TrackedDataHandlerRegistry.CAT_VARIANT) {
            return new Pair<>("cat_variant",
                    new JsonPrimitive(Registries.CAT_VARIANT.getId((CatVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.FROG_VARIANT) {
            return new Pair<>("frog_variant",
                    new JsonPrimitive(Registries.FROG_VARIANT.getId((FrogVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_GLOBAL_POS) {
            return new Pair<>("optional_global_pos", ((Optional<?>) val).map(o -> {
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
            return new Pair<>("painting_variant", new JsonPrimitive(variant));
        } else if (handler == TrackedDataHandlerRegistry.SNIFFER_STATE) {
            return new Pair<>("sniffer_state",
                    new JsonPrimitive(((SnifferEntity.State) val).name().toLowerCase(Locale.ROOT)));
        } else if (handler == TrackedDataHandlerRegistry.VECTOR3F) {
            var vec = (Vector3f) val;
            var json = new JsonObject();
            json.addProperty("x", vec.x);
            json.addProperty("y", vec.y);
            json.addProperty("z", vec.z);
            return new Pair<>("vector3f", json);
        } else if (handler == TrackedDataHandlerRegistry.QUATERNIONF) {
            var quat = (Quaternionf) val;
            var json = new JsonObject();
            json.addProperty("x", quat.x);
            json.addProperty("y", quat.y);
            json.addProperty("z", quat.z);
            json.addProperty("w", quat.w);
            return new Pair<>("quaternionf", json);
        } else {
            throw new IllegalArgumentException(
                    "Unexpected tracked handler of ID " + TrackedDataHandlerRegistry.getId(handler));
        }
    }

    @Override
    public String fileName() {
        return "entities.json";
    }

    @Override
    @SuppressWarnings("unchecked")
    public void extract(MinecraftServer server, DataOutput output, Gson gson)
            throws IllegalAccessException, NoSuchFieldException, IOException {

        final var entityClassToType = new HashMap<Class<? extends Entity>, EntityType<?>>();
        for (var f : EntityType.class.getFields()) {
            if (f.getType().equals(EntityType.class)) {
                var entityClass = (Class<? extends Entity>) ((ParameterizedType) f.getGenericType())
                        .getActualTypeArguments()[0];
                var entityType = (EntityType<?>) f.get(null);

                entityClassToType.put(entityClass, entityType);
            }
        }

        final var dataTrackerField = Entity.class.getDeclaredField("dataTracker");
        dataTrackerField.setAccessible(true);

        var entitiesMap = new TreeMap<Class<? extends Entity>, JsonElement>(new ClassComparator());

        for (var entry : entityClassToType.entrySet()) {
            var entityClass = entry.getKey();
            @Nullable
            var entityType = entry.getValue();
            assert entityType != null;

            // While we can use the tracked data registry and reflection to get the tracked
            // fields on entities, we won't know what their default values are because they
            // are assigned in the entity's constructor.
            // To obtain this, we create a dummy world to spawn the entities into and read
            // the data tracker field from the base entity class.
            // We also handle player entities specially since they cannot be spawned with
            // EntityType#create.
            final var entityInstance = entityType.equals(EntityType.PLAYER)
                    ? DummyPlayerEntity.INSTANCE
                    : entityType.create(DummyWorld.INSTANCE);

            final var dataTracker = entityInstance.getDataTracker();

            while (entitiesMap.get(entityClass) == null) {
                var entityJson = new JsonObject();

                var parent = entityClass.getSuperclass();
                var hasParent = parent != null && Entity.class.isAssignableFrom(parent);

                if (hasParent) {
                    entityJson.addProperty("parent", parent.getSimpleName());
                }

                if (entityType != null) {
                    entityJson.addProperty("type", Registries.ENTITY_TYPE.getId(entityType).getPath());

                    entityJson.add("translation_key", new JsonPrimitive(entityType.getTranslationKey()));
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

                        var data = Entities.trackedDataToJson(trackedData, dataTracker);
                        fieldJson.addProperty("type", data.left());
                        fieldJson.add("default_value", data.right());

                        fieldsJson.add(fieldJson);
                    }
                }
                entityJson.add("fields", fieldsJson);

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
                if (bb != null && entityType != null) {
                    var boundingBoxJson = new JsonObject();

                    boundingBoxJson.addProperty("size_x", bb.getLengthX());
                    boundingBoxJson.addProperty("size_y", bb.getLengthY());
                    boundingBoxJson.addProperty("size_z", bb.getLengthZ());

                    entityJson.add("default_bounding_box", boundingBoxJson);
                }

                entitiesMap.put(entityClass, entityJson);

                if (!hasParent) {
                    break;
                }

                entityClass = (Class<? extends Entity>) parent;
                entityType = entityClassToType.get(entityClass);
            }
        }

        var entitiesJson = new JsonObject();
        for (var entry : entitiesMap.entrySet()) {
            entitiesJson.add(entry.getKey().getSimpleName(), entry.getValue());
        }

        Main.writeJson(output, gson, entitiesJson);
    }
}
