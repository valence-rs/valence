package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.fabricmc.fabric.impl.biome.modification.BuiltInRegistryKeys;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.entity.attribute.ClampedEntityAttribute;
import net.minecraft.entity.attribute.EntityAttribute;
import net.minecraft.entity.data.TrackedDataHandler;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.SnifferEntity;
import net.minecraft.network.packet.s2c.play.EntityAnimationS2CPacket;
import net.minecraft.registry.BuiltinRegistries;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryBuilder;
import net.minecraft.registry.RegistryWrapper;
import net.minecraft.util.math.Direction;
import rs.valence.extractor.Main;
import java.lang.reflect.Modifier;
import java.util.Locale;

public class Misc implements Main.Extractor {
    @Override
    public String fileName() {
        return "misc.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        final var miscJson = new JsonObject();

        final var entityTypeJson = new JsonObject();
        for (final var type : Registries.ENTITY_TYPE) {
            entityTypeJson.addProperty(Registries.ENTITY_TYPE.getId(type).getPath(),
                    Registries.ENTITY_TYPE.getRawId(type));
        }
        miscJson.add("entity_type", entityTypeJson);

        final var entityStatusJson = new JsonObject();
        for (final var field : EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof final Byte code) {
                if ("field_30030".equals(field.getName())) {
                    entityStatusJson.addProperty("stop_attack", code);
                } else {
                    entityStatusJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
                }
            }
        }
        miscJson.add("entity_status", entityStatusJson);

        final var entityAnimationJson = new JsonObject();
        for (final var field : EntityAnimationS2CPacket.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.canAccess(null)
                    && field.get(null) instanceof final Integer i) {
                entityAnimationJson.addProperty(field.getName().toLowerCase(Locale.ROOT), i);
            }
        }
        miscJson.add("entity_animation", entityAnimationJson);

        final var villagerTypeJson = new JsonObject();
        for (final var type : Registries.VILLAGER_TYPE) {
            villagerTypeJson.addProperty(Registries.VILLAGER_TYPE.getId(type).getPath(),
                    Registries.VILLAGER_TYPE.getRawId(type));
        }
        miscJson.add("villager_type", villagerTypeJson);

        final var villagerProfessionJson = new JsonObject();
        for (final var profession : Registries.VILLAGER_PROFESSION) {
            villagerProfessionJson.addProperty(profession.id(), Registries.VILLAGER_PROFESSION.getRawId(profession));
        }
        miscJson.add("villager_profession", villagerProfessionJson);

        final var catVariantJson = new JsonObject();
        for (final var variant : Registries.CAT_VARIANT) {
            catVariantJson.addProperty(Registries.CAT_VARIANT.getId(variant).getPath(),
                    Registries.CAT_VARIANT.getRawId(variant));
        }
        miscJson.add("cat_variant", catVariantJson);

        final var frogVariantJson = new JsonObject();
        for (final var variant : Registries.FROG_VARIANT) {
            frogVariantJson.addProperty(Registries.FROG_VARIANT.getId(variant).getPath(),
                    Registries.FROG_VARIANT.getRawId(variant));
        }
        miscJson.add("frog_variant", frogVariantJson);



        final var directionJson = new JsonObject();
        for (final var dir : Direction.values()) {
            directionJson.addProperty(dir.getName(), dir.getId());
        }
        miscJson.add("direction", directionJson);

        final var entityPoseJson = new JsonObject();
        final var poses = EntityPose.values();
        for (int i = 0; i < poses.length; i++) {
            entityPoseJson.addProperty(poses[i].name().toLowerCase(Locale.ROOT), i);
        }
        miscJson.add("entity_pose", entityPoseJson);

        final var particleTypesJson = new JsonObject();
        for (final var type : Registries.PARTICLE_TYPE) {
            particleTypesJson.addProperty(Registries.PARTICLE_TYPE.getId(type).getPath(),
                    Registries.PARTICLE_TYPE.getRawId(type));
        }
        miscJson.add("particle_type", particleTypesJson);

        final var snifferStateJson = new JsonObject();
        for (final var state : SnifferEntity.State.values()) {
            snifferStateJson.addProperty(state.name().toLowerCase(Locale.ROOT), state.ordinal());
        }
        miscJson.add("sniffer_state", snifferStateJson);

        final var trackedDataHandlerJson = new JsonObject();
        for (final var field : TrackedDataHandlerRegistry.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.get(null) instanceof final TrackedDataHandler<?> handler) {
                final var name = field.getName().toLowerCase(Locale.ROOT);
                final var id = TrackedDataHandlerRegistry.getId(handler);

                trackedDataHandlerJson.addProperty(name, id);
            }
        }
        miscJson.add("tracked_data_handler", trackedDataHandlerJson);

        return miscJson;
    }
}
