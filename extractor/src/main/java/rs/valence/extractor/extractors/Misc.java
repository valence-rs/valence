package rs.valence.extractor.extractors;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.entity.data.TrackedDataHandler;
import net.minecraft.entity.data.TrackedDataHandlerRegistry;
import net.minecraft.entity.passive.SnifferEntity;
import net.minecraft.network.packet.s2c.play.EntityAnimationS2CPacket;
import net.minecraft.registry.Registries;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.math.Direction;
import rs.valence.extractor.Main;

import java.io.DataOutput;
import java.lang.reflect.Modifier;
import java.util.Locale;

public class Misc implements Main.Extractor {
    @Override
    public String fileName() {
        return "misc.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson) throws Exception {
        var miscJson = new JsonObject();

        var entityTypeJson = new JsonObject();
        for (var type : Registries.ENTITY_TYPE) {
            entityTypeJson.addProperty(Registries.ENTITY_TYPE.getId(type).getPath(),
                    Registries.ENTITY_TYPE.getRawId(type));
        }
        miscJson.add("entity_type", entityTypeJson);

        var entityStatusJson = new JsonObject();
        for (var field : EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                if (field.getName().equals("field_30030")) {
                    entityStatusJson.addProperty("stop_attack", code);
                } else {
                    entityStatusJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
                }
            }
        }
        miscJson.add("entity_status", entityStatusJson);

        var entityAnimationJson = new JsonObject();
        for (var field : EntityAnimationS2CPacket.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.canAccess(null)
                    && field.get(null) instanceof Integer i) {
                entityAnimationJson.addProperty(field.getName().toLowerCase(Locale.ROOT), i);
            }
        }
        miscJson.add("entity_animation", entityAnimationJson);

        var villagerTypeJson = new JsonObject();
        for (var type : Registries.VILLAGER_TYPE) {
            villagerTypeJson.addProperty(Registries.VILLAGER_TYPE.getId(type).getPath(),
                    Registries.VILLAGER_TYPE.getRawId(type));
        }
        miscJson.add("villager_type", villagerTypeJson);

        var villagerProfessionJson = new JsonObject();
        for (var profession : Registries.VILLAGER_PROFESSION) {
            villagerProfessionJson.addProperty(profession.id(), Registries.VILLAGER_PROFESSION.getRawId(profession));
        }
        miscJson.add("villager_profession", villagerProfessionJson);

        var catVariantJson = new JsonObject();
        for (var variant : Registries.CAT_VARIANT) {
            catVariantJson.addProperty(Registries.CAT_VARIANT.getId(variant).getPath(),
                    Registries.CAT_VARIANT.getRawId(variant));
        }
        miscJson.add("cat_variant", catVariantJson);

        var frogVariantJson = new JsonObject();
        for (var variant : Registries.FROG_VARIANT) {
            frogVariantJson.addProperty(Registries.FROG_VARIANT.getId(variant).getPath(),
                    Registries.FROG_VARIANT.getRawId(variant));
        }
        miscJson.add("frog_variant", frogVariantJson);

        var paintingVariantJson = new JsonObject();
        for (var variant : Registries.PAINTING_VARIANT) {
            var variantJson = new JsonObject();
            variantJson.addProperty("id", Registries.PAINTING_VARIANT.getRawId(variant));
            variantJson.addProperty("width", variant.getWidth());
            variantJson.addProperty("height", variant.getHeight());
            paintingVariantJson.add(Registries.PAINTING_VARIANT.getId(variant).getPath(), variantJson);
        }
        miscJson.add("painting_variant", paintingVariantJson);

        var directionJson = new JsonObject();
        for (var dir : Direction.values()) {
            directionJson.addProperty(dir.getName(), dir.getId());
        }
        miscJson.add("direction", directionJson);

        var entityPoseJson = new JsonObject();
        var poses = EntityPose.values();
        for (int i = 0; i < poses.length; i++) {
            entityPoseJson.addProperty(poses[i].name().toLowerCase(Locale.ROOT), i);
        }
        miscJson.add("entity_pose", entityPoseJson);

        var particleTypesJson = new JsonObject();
        for (var type : Registries.PARTICLE_TYPE) {
            particleTypesJson.addProperty(Registries.PARTICLE_TYPE.getId(type).getPath(),
                    Registries.PARTICLE_TYPE.getRawId(type));
        }
        miscJson.add("particle_type", particleTypesJson);

        var snifferStateJson = new JsonObject();
        for (var state : SnifferEntity.State.values()) {
            snifferStateJson.addProperty(state.name().toLowerCase(Locale.ROOT), state.ordinal());
        }
        miscJson.add("sniffer_state", snifferStateJson);

        var trackedDataHandlerJson = new JsonObject();
        for (var field : TrackedDataHandlerRegistry.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.get(null) instanceof TrackedDataHandler<?> handler) {
                var name = field.getName().toLowerCase(Locale.ROOT);
                var id = TrackedDataHandlerRegistry.getId(handler);

                trackedDataHandlerJson.addProperty(name, id);
            }
        }
        miscJson.add("tracked_data_handler", trackedDataHandlerJson);

        Main.writeJson(output, gson, miscJson);
    }
}
