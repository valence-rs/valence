package dev._00a.valence_extractor.extractors;

import com.google.gson.*;
import dev._00a.valence_extractor.DummyPlayerEntity;
import dev._00a.valence_extractor.DummyWorld;
import dev._00a.valence_extractor.Main;
import dev._00a.valence_extractor.Main.Pair;
import net.minecraft.entity.Entity;
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

import java.lang.reflect.ParameterizedType;
import java.util.HashSet;
import java.util.Locale;
import java.util.Optional;
import java.util.OptionalInt;

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
        } else if (handler == TrackedDataHandlerRegistry.FLOAT) {
            return new Pair<>("float", new JsonPrimitive((Float) val));
        } else if (handler == TrackedDataHandlerRegistry.STRING) {
            return new Pair<>("string", new JsonPrimitive((String) val));
        } else if (handler == TrackedDataHandlerRegistry.TEXT_COMPONENT) {
            // TODO: return text as json element.
            return new Pair<>("text_component", new JsonPrimitive(((Text) val).getString()));
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_TEXT_COMPONENT) {
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(((Text) o).getString())).orElse(JsonNull.INSTANCE);
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
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_uuid", res);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_BLOCK_STATE) {
            // TODO: get raw block state ID.
            var res = ((Optional<?>) val).map(o -> (JsonElement) new JsonPrimitive(o.toString())).orElse(JsonNull.INSTANCE);
            return new Pair<>("optional_block_state", res);
        } else if (handler == TrackedDataHandlerRegistry.NBT_COMPOUND) {
            // TODO: base64 binary representation or SNBT?
            return new Pair<>("nbt_compound", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.PARTICLE) {
            return new Pair<>("particle", new JsonPrimitive(((ParticleEffect) val).asString()));
        } else if (handler == TrackedDataHandlerRegistry.VILLAGER_DATA) {
            var vd = (VillagerData) val;
            var json = new JsonObject();
            json.addProperty("type", vd.getType().toString());
            json.addProperty("profession", vd.getProfession().toString());
            json.addProperty("level", vd.getLevel());
            return new Pair<>("villager_data", json);
        } else if (handler == TrackedDataHandlerRegistry.OPTIONAL_INT) {
            var opt = (OptionalInt) val;
            return new Pair<>("optional_int", opt.isPresent() ? new JsonPrimitive(opt.getAsInt()) : JsonNull.INSTANCE);
        } else if (handler == TrackedDataHandlerRegistry.ENTITY_POSE) {
            return new Pair<>("entity_pose", new JsonPrimitive(val.toString()));
        } else if (handler == TrackedDataHandlerRegistry.CAT_VARIANT) {
            return new Pair<>("cat_variant", new JsonPrimitive(Registry.CAT_VARIANT.getId((CatVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.FROG_VARIANT) {
            return new Pair<>("frog_variant", new JsonPrimitive(Registry.FROG_VARIANT.getId((FrogVariant) val).getPath()));
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
        } else {
            throw new IllegalArgumentException("Unexpected tracked data type " + handler);
        }
    }

    @Override
    public String fileName() {
        return "entities.json";
    }

    @Override
    @SuppressWarnings("unchecked")
    public JsonElement extract() throws IllegalAccessException, NoSuchFieldException {
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
                            var res = Entities.trackedDataToJson(data, dataTracker);
                            fieldJson.addProperty("type", res.left());
                            fieldJson.add("default_value", res.right());

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

        return entitiesJson;
    }
}
