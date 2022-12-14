package rs.valence.extractor.extractors;

import com.google.gson.*;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityType;
import net.minecraft.entity.data.DataTracker;
import net.minecraft.entity.data.TrackedData;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.CatVariant;
import net.minecraft.entity.passive.FrogVariant;
import net.minecraft.item.ItemStack;
import net.minecraft.particle.ParticleEffect;
import net.minecraft.registry.Registries;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.text.Text;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.EulerAngle;
import net.minecraft.util.math.GlobalPos;
import net.minecraft.village.VillagerData;
import org.jetbrains.annotations.Nullable;
import rs.valence.extractor.ClassComparator;
import rs.valence.extractor.DummyPlayerEntity;
import rs.valence.extractor.DummyWorld;
import rs.valence.extractor.Main;
import rs.valence.extractor.Main.Pair;

import java.lang.reflect.ParameterizedType;
import java.util.*;

public class Entities implements Main.Extractor {
    private final static Map<String, Bit[]> BIT_FIELDS = Map.ofEntries(
            // @formatter:off
            bits(
                    "flags",
                    bit("on_fire", 0),
                    bit("sneaking", 1),
                    bit("sprinting", 3),
                    bit("swimming", 4),
                    bit("invisible", 5),
                    bit("glowing", 6),
                    bit("fall_flying", 7)
            ),
            bits(
                    "projectile_flags",
                    bit("critical", 0),
                    bit("no_clip", 1)
            ),
            bits(
                    "living_flags",
                    bit("using_item", 0),
                    bit("off_hand_active", 1),
                    bit("using_riptide", 2)
            ),
            bits(
                    "player_model_parts",
                    bit("cape", 0),
                    bit("jacket", 1),
                    bit("left_sleeve", 2),
                    bit("right_sleeve", 3),
                    bit("left_pants_leg", 4),
                    bit("right_pants_leg", 5),
                    bit("hat", 6)
            ),
            bits(
                    "armor_stand_flags",
                    bit("small", 0),
                    bit("show_arms", 1),
                    bit("hide_base_plate", 2),
                    bit("marker", 3)
            ),
            bits(
                    "mob_flags",
                    bit("ai_disabled", 0),
                    bit("left_handed", 1),
                    bit("attacking", 2)
            ),
            bits(
                    "bat_flags",
                    bit("hanging", 0)
            ),
            bits(
                    "horse_flags",
                    bit("tamed", 1),
                    bit("saddled", 2),
                    bit("bred", 3),
                    bit("eating_grass", 4),
                    bit("angry", 5),
                    bit("eating", 6)
            ),
            bits(
                    "bee_flags",
                    bit("near_target", 1),
                    bit("has_stung", 2),
                    bit("has_nectar", 3)
            ),
            bits(
                    "fox_flags",
                    bit("sitting", 0),
                    bit("crouching", 2),
                    bit("rolling_head", 3),
                    bit("chasing", 4),
                    bit("sleeping", 5),
                    bit("walking", 6),
                    bit("aggressive", 7)
            ),
            bits(
                    "panda_flags",
                    bit("sneezing", 1),
                    bit("playing", 2),
                    bit("sitting", 3),
                    bit("lying_on_back", 4)
            ),
            bits(
                    "tameable_flags",
                    bit("sitting_pose", 0),
                    bit("tamed", 2)
            ),
            bits(
                    "iron_golem_flags",
                    bit("player_created", 0)
            ),
            bits(
                    "snow_golem_flags",
                    bit("has_pumpkin", 4)
            ),
            bits(
                    "blaze_flags",
                    bit("fire_active", 0)
            ),
            bits(
                    "vex_flags",
                    bit("charging", 0)
            ),
            bits(
                    "spider_flags",
                    bit("climbing_wall", 0)
            )
            // @formatter:on
    );

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
            return new Pair<>("cat_variant", new JsonPrimitive(Registries.CAT_VARIANT.getId((CatVariant) val).getPath()));
        } else if (handler == TrackedDataHandlerRegistry.FROG_VARIANT) {
            return new Pair<>("frog_variant", new JsonPrimitive(Registries.FROG_VARIANT.getId((FrogVariant) val).getPath()));
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
            throw new IllegalArgumentException("Unexpected tracked data type");
        }
    }

    private static Bit bit(String name, int index) {
        return new Bit(name, index);
    }

    private static Map.Entry<String, Bit[]> bits(String fieldName, Bit... bits) {
        return Map.entry(fieldName, bits);
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

        final var dataTrackerField = Entity.class.getDeclaredField("dataTracker");
        dataTrackerField.setAccessible(true);

        var entitiesMap = new TreeMap<Class<? extends Entity>, JsonElement>(new ClassComparator());

        for (var entry : entityClassToType.entrySet()) {
            var entityClass = entry.getKey();
            @Nullable var entityType = entry.getValue();
            assert entityType != null;

            // While we can use the tracked data registry and reflection to get the tracked fields on entities, we won't know what their default values are because they are assigned in the entity's constructor.
            // To obtain this, we create a dummy world to spawn the entities into and read the data tracker field from the base entity class.
            // We also handle player entities specially since they cannot be spawned with EntityType#create.
            final var entityInstance = entityType.equals(EntityType.PLAYER) ? DummyPlayerEntity.INSTANCE : entityType.create(DummyWorld.INSTANCE);

            final var dataTracker = (DataTracker) dataTrackerField.get(entityInstance);

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

                        var bitsJson = new JsonArray();
                        for (var bit : BIT_FIELDS.getOrDefault(fieldName, new Bit[]{})) {
                            var bitJson = new JsonObject();
                            bitJson.addProperty("name", bit.name);
                            bitJson.addProperty("index", bit.index);
                            bitsJson.add(bitJson);
                        }
                        fieldJson.add("bits", bitsJson);

                        fieldsJson.add(fieldJson);
                    }
                }
                entityJson.add("fields", fieldsJson);

                var bb = entityInstance.getBoundingBox();
                if (bb != null) {
                    var boundingBoxJson = new JsonObject();

                    boundingBoxJson.addProperty("size_x", bb.getXLength());
                    boundingBoxJson.addProperty("size_y", bb.getYLength());
                    boundingBoxJson.addProperty("size_z", bb.getZLength());

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
