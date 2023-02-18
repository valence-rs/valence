package rs.valence.extractor.extractors;

import com.google.gson.*;
import net.minecraft.core.BlockPos;
import net.minecraft.core.GlobalPos;
import net.minecraft.core.Holder;
import net.minecraft.core.Rotations;
import net.minecraft.core.particles.ParticleOptions;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.Pose;
import net.minecraft.world.entity.animal.CatVariant;
import net.minecraft.world.entity.animal.FrogVariant;
import net.minecraft.world.entity.npc.VillagerData;
import net.minecraft.world.item.ItemStack;
import org.jetbrains.annotations.Nullable;
import rs.valence.extractor.ClassComparator;
import rs.valence.extractor.MockLevel;
import rs.valence.extractor.MockPlayerEntity;
import rs.valence.extractor.Main;
import rs.valence.extractor.Main.Pair;

import java.lang.reflect.ParameterizedType;
import java.util.*;

public class Entities implements Main.Extractor {
    public Entities() {
    }

    private static Pair<String, JsonElement> synchedDataToJson(EntityDataAccessor<?> accessor, SynchedEntityData data) {
        final var ser = accessor.getSerializer();
        final var val = data.get(accessor);

        if (ser == EntityDataSerializers.BYTE) {
            return new Pair<>("byte", new JsonPrimitive((Byte) val));
        } else if (ser == EntityDataSerializers.INT) {
            return new Pair<>("integer", new JsonPrimitive((Integer) val));
        } else if (ser == EntityDataSerializers.LONG) {
            return new Pair<>("long", new JsonPrimitive((Long) val));
        } else if (ser == EntityDataSerializers.FLOAT) {
            return new Pair<>("float", new JsonPrimitive((Float) val));
        } else if (ser == EntityDataSerializers.STRING) {
            return new Pair<>("string", new JsonPrimitive((String) val));
        } else if (ser == EntityDataSerializers.COMPONENT) {
            // TODO: return text as json element.
            return new Pair<>("component", new JsonPrimitive(((Component) val).getString()));
        } else if (ser == EntityDataSerializers.OPTIONAL_COMPONENT) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(((Component) o).getString())).orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_component", res);
        } else if (ser == EntityDataSerializers.ITEM_STACK) {
            // TODO
            return new Pair<>("item_stack", new JsonPrimitive(((ItemStack) val).toString()));
        } else if (ser == EntityDataSerializers.BOOLEAN) {
            return new Pair<>("boolean", new JsonPrimitive((Boolean) val));
        } else if (ser == EntityDataSerializers.ROTATIONS) {
            var json = new JsonObject();
            var rot = (Rotations) val;
            json.addProperty("pitch", rot.getX());
            json.addProperty("yaw", rot.getY());
            json.addProperty("roll", rot.getZ());
            return new Pair<>("rotation", json);
        } else if (ser == EntityDataSerializers.BLOCK_POS) {
            var bp = (BlockPos) val;
            var json = new JsonObject();
            json.addProperty("x", bp.getX());
            json.addProperty("y", bp.getY());
            json.addProperty("z", bp.getZ());
            return new Pair<>("block_pos", json);
        } else if (ser == EntityDataSerializers.OPTIONAL_BLOCK_POS) {
            return new Pair<>("optional_block_pos", ((Optional<?>) val).map(o -> {
                var bp = (BlockPos) o;
                var json = new JsonObject();
                json.addProperty("x", bp.getX());
                json.addProperty("y", bp.getY());
                json.addProperty("z", bp.getZ());
                return (JsonElement) json;
            }).orElse(JsonNull.INSTANCE));
        } else if (ser == EntityDataSerializers.DIRECTION) {
            return new Pair<>("direction", new JsonPrimitive(val.toString()));
        } else if (ser == EntityDataSerializers.OPTIONAL_UUID) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_uuid", res);
        } else if (ser == EntityDataSerializers.BLOCK_STATE) {
            // TODO: get raw block state ID.
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new Pair<>("block_state", res);
        } else if (ser == EntityDataSerializers.COMPOUND_TAG) {
            // TODO: base64 binary representation or SNBT?
            return new Pair<>("compound_tag", new JsonPrimitive(val.toString()));
        } else if (ser == EntityDataSerializers.PARTICLE) {
            var key = BuiltInRegistries.PARTICLE_TYPE.getKey(((ParticleOptions) val).getType());
            return new Pair<>("particle", new JsonPrimitive(key.getPath()));
        } else if (ser == EntityDataSerializers.VILLAGER_DATA) {
            var vd = (VillagerData) val;
            var json = new JsonObject();
            var type = BuiltInRegistries.VILLAGER_TYPE.getKey(vd.getType()).getPath();
            var profession = BuiltInRegistries.VILLAGER_PROFESSION.getKey(vd.getProfession()).getPath();
            json.addProperty("type", type);
            json.addProperty("profession", profession);
            json.addProperty("level", vd.getLevel());
            return new Pair<>("villager_data", json);
        } else if (ser == EntityDataSerializers.OPTIONAL_UNSIGNED_INT) {
            var opt = (OptionalInt) val;
            return new Pair<>("optional_unsigned_int", opt.isPresent() ? new JsonPrimitive(opt.getAsInt()) : JsonNull.INSTANCE);
        } else if (ser == EntityDataSerializers.POSE) {
            return new Pair<>("entity_pose", new JsonPrimitive(((Pose) val).name().toLowerCase(Locale.ROOT)));
        } else if (ser == EntityDataSerializers.CAT_VARIANT) {
            return new Pair<>("cat_variant", new JsonPrimitive(BuiltInRegistries.CAT_VARIANT.getKey((CatVariant) val).getPath()));
        } else if (ser == EntityDataSerializers.FROG_VARIANT) {
            return new Pair<>("frog_variant", new JsonPrimitive(BuiltInRegistries.FROG_VARIANT.getKey((FrogVariant) val).getPath()));
        } else if (ser == EntityDataSerializers.OPTIONAL_GLOBAL_POS) {
            return new Pair<>("optional_global_pos", ((Optional<?>) val).map(o -> {
                var gp = (GlobalPos) o;
                var json = new JsonObject();
                json.addProperty("dimension", gp.dimension().location().toString());

                var posJson = new JsonObject();
                posJson.addProperty("x", gp.pos().getX());
                posJson.addProperty("y", gp.pos().getY());
                posJson.addProperty("z", gp.pos().getZ());

                json.add("position", posJson);
                return (JsonElement) json;
            }).orElse(JsonNull.INSTANCE));
        } else if (ser == EntityDataSerializers.PAINTING_VARIANT) {
            var variant = ((Holder<?>) val).unwrapKey().map(k -> k.location().getPath()).orElse("");
            return new Pair<>("painting_variant", new JsonPrimitive(variant));
        } else {
            throw new IllegalArgumentException("Unexpected tracked data type");
        }
    }

    @Override
    public String fileName() {
        return "entities.json";
    }

    @Override
    @SuppressWarnings("unchecked")
    public JsonElement extract() throws IllegalAccessException, NoSuchFieldException {

        final var entityClassToType = new HashMap<Class<? extends Entity>, EntityType<?>>();
        for (var f : EntityType.class.getFields()) {
            if (f.getType().equals(EntityType.class)) {
                var entityClass = (Class<? extends Entity>) ((ParameterizedType) f.getGenericType()).getActualTypeArguments()[0];
                var entityType = (EntityType<?>) f.get(null);

                entityClassToType.put(entityClass, entityType);
            }
        }

        final var synchedDataField = Entity.class.getDeclaredField("entityData");
        synchedDataField.setAccessible(true);

        var entitiesMap = new TreeMap<Class<? extends Entity>, JsonElement>(new ClassComparator());

        for (var entry : entityClassToType.entrySet()) {
            var entityClass = entry.getKey();
            @Nullable var entityType = entry.getValue();
            assert entityType != null;

            // While we can use reflection to get the synched data fields on entities, we won't know what their default values are because they are assigned in the entity's constructor.
            // To obtain this, we create a dummy world to spawn the entities into and read the synched entity data field from the base entity class.
            // We also handle player entities specially since they cannot be spawned with EntityType#create.
            final Entity entityInstance = entityType.equals(EntityType.PLAYER) ? MockPlayerEntity.INSTANCE : entityType.create(MockLevel.INSTANCE);

            final var synchedData = (SynchedEntityData) synchedDataField.get(entityInstance);

            while (entitiesMap.get(entityClass) == null) {
                var entityJson = new JsonObject();

                var parent = entityClass.getSuperclass();
                var hasParent = parent != null && Entity.class.isAssignableFrom(parent);

                if (hasParent) {
                    entityJson.addProperty("parent", parent.getSimpleName());
                }

                if (entityType != null) {
                    entityJson.addProperty("type", BuiltInRegistries.ENTITY_TYPE.getKey(entityType).getPath());

                    entityJson.add("translation_key", new JsonPrimitive(entityType.getDescriptionId()));
                }

                var fieldsJson = new JsonArray();
                for (var entityField : entityClass.getDeclaredFields()) {
                    if (entityField.getType().equals(EntityDataAccessor.class)) {
                        entityField.setAccessible(true);

                        var trackedData = (EntityDataAccessor<?>) entityField.get(null);

                        var fieldJson = new JsonObject();
                        var fieldName = entityField.getName().toLowerCase(Locale.ROOT);
                        fieldJson.addProperty("name", fieldName);
                        fieldJson.addProperty("index", trackedData.getId());

                        var data = Entities.synchedDataToJson(trackedData, synchedData);
                        fieldJson.addProperty("type", data.left());
                        fieldJson.add("default_value", data.right());

                        fieldsJson.add(fieldJson);
                    }
                }
                entityJson.add("fields", fieldsJson);

                var bb = entityInstance.getBoundingBox();
                if (bb != null) {
                    var boundingBoxJson = new JsonObject();

                    boundingBoxJson.addProperty("size_x", bb.getXsize());
                    boundingBoxJson.addProperty("size_y", bb.getYsize());
                    boundingBoxJson.addProperty("size_z", bb.getZsize());

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

        return entitiesJson;
    }

    private record Bit(String name, int index) {
    }
}
