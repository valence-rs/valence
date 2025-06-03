package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.lang.reflect.Modifier;
import java.util.Locale;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.entity.data.TrackedDataHandler;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.ArmadilloEntity;
import net.minecraft.entity.passive.SnifferEntity;
import net.minecraft.network.packet.s2c.play.EntityAnimationS2CPacket;
import net.minecraft.registry.*;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.math.Direction;
import rs.valence.extractor.Main;

public class Misc implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;

    public Misc(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    @Override
    public String fileName() {
        return "misc.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var miscJson = new JsonObject();

        var entityTypeJson = new JsonObject();
        for (var type : Registries.ENTITY_TYPE) {
            entityTypeJson.addProperty(
                Registries.ENTITY_TYPE.getId(type).getPath(),
                Registries.ENTITY_TYPE.getRawId(type)
            );
        }
        miscJson.add("entity_type", entityTypeJson);

        var entityStatusJson = new JsonObject();
        for (var field : EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                if ("field_30030".equals(field.getName())) {
                    entityStatusJson.addProperty("stop_attack", code);
                } else {
                    entityStatusJson.addProperty(
                        field.getName().toLowerCase(Locale.ROOT),
                        code
                    );
                }
            }
        }
        miscJson.add("entity_status", entityStatusJson);

        var entityAnimationJson = new JsonObject();
        for (var field : EntityAnimationS2CPacket.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (
                Modifier.isStatic(field.getModifiers()) &&
                field.canAccess(null) &&
                field.get(null) instanceof Integer i
            ) {
                entityAnimationJson.addProperty(
                    field.getName().toLowerCase(Locale.ROOT),
                    i
                );
            }
        }
        miscJson.add("entity_animation", entityAnimationJson);

        var villagerTypeJson = new JsonObject();
        for (var type : Registries.VILLAGER_TYPE) {
            villagerTypeJson.addProperty(
                Registries.VILLAGER_TYPE.getId(type).getPath(),
                Registries.VILLAGER_TYPE.getRawId(type)
            );
        }
        miscJson.add("villager_type", villagerTypeJson);

        var villagerProfessionJson = new JsonObject();
        for (var profession : Registries.VILLAGER_PROFESSION) {
            villagerProfessionJson.addProperty(
                profession.id().getString().toLowerCase(),
                Registries.VILLAGER_PROFESSION.getRawId(profession)
            );
        }
        miscJson.add("villager_profession", villagerProfessionJson);

        var catVariantJson = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.CAT_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.CAT_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.CAT_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("cat_variant", catVariantJson);

        var frogVariantJson = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.FROG_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.FROG_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.FROG_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("frog_variant", frogVariantJson);

        var wolfVariantJson = new JsonObject();
        for (var variant : registryManager.getOrThrow(
            RegistryKeys.WOLF_VARIANT
        )) {
            wolfVariantJson.addProperty(
                registryManager
                    .getOrThrow(RegistryKeys.WOLF_VARIANT)
                    .getId(variant)
                    .getPath(),
                registryManager
                    .getOrThrow(RegistryKeys.WOLF_VARIANT)
                    .getRawId(variant)
            );
        }
        miscJson.add("wolf_variant", wolfVariantJson);

        var pigVariant = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.PIG_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.PIG_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.PIG_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("pig_variant", pigVariant);

        var cowVariant = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.COW_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.COW_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.COW_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("cow_variant", cowVariant);

        var chickenVariant = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.CHICKEN_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.CHICKEN_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.CHICKEN_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("chicken_variant", chickenVariant);

        var paintingVariant = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.PAINTING_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.PAINTING_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.PAINTING_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("painting_variant", paintingVariant);

        var wolfSoundVariant = new JsonObject();
        for (var variant : registryManager.getOrThrow(
                RegistryKeys.WOLF_SOUND_VARIANT
        )) {
            catVariantJson.addProperty(
                    registryManager
                            .getOrThrow(RegistryKeys.WOLF_SOUND_VARIANT)
                            .getId(variant)
                            .getPath(),
                    registryManager
                            .getOrThrow(RegistryKeys.WOLF_SOUND_VARIANT)
                            .getRawId(variant)
            );
        }
        miscJson.add("wolf_sound_variant", wolfSoundVariant);

        var directionJson = new JsonObject();
        for (var dir : Direction.values()) {
            directionJson.addProperty(dir.name(), dir.getId());
        }
        miscJson.add("direction", directionJson);

        var entityPoseJson = new JsonObject();
        var poses = EntityPose.values();
        for (int i = 0; i < poses.length; i++) {
            entityPoseJson.addProperty(
                poses[i].name().toLowerCase(Locale.ROOT),
                i
            );
        }
        miscJson.add("entity_pose", entityPoseJson);

        var particleTypesJson = new JsonObject();
        for (var type : Registries.PARTICLE_TYPE) {
            particleTypesJson.addProperty(
                Registries.PARTICLE_TYPE.getId(type).getPath(),
                Registries.PARTICLE_TYPE.getRawId(type)
            );
        }
        miscJson.add("particle_type", particleTypesJson);

        var snifferStateJson = new JsonObject();
        for (var state : SnifferEntity.State.values()) {
            snifferStateJson.addProperty(
                state.name().toLowerCase(Locale.ROOT),
                state.ordinal()
            );
        }
        miscJson.add("sniffer_state", snifferStateJson);

        var armadilloStateJson = new JsonObject();
        for (var state : ArmadilloEntity.State.values()) {
            armadilloStateJson.addProperty(
                state.name().toLowerCase(Locale.ROOT),
                state.ordinal()
            );
        }
        miscJson.add("armadillo_state", armadilloStateJson);

        var trackedDataHandlerJson = new JsonObject();
        for (var field : TrackedDataHandlerRegistry.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (
                Modifier.isStatic(field.getModifiers()) &&
                field.get(null) instanceof TrackedDataHandler<?> handler
            ) {
                var name = field.getName().toLowerCase(Locale.ROOT);
                var id = TrackedDataHandlerRegistry.getId(handler);

                trackedDataHandlerJson.addProperty(name, id);
            }
        }
        miscJson.add("tracked_data_handler", trackedDataHandlerJson);

        return miscJson;
    }
}
