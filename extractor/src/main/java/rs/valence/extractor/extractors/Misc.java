package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.protocol.game.ClientboundAnimatePacket;
import net.minecraft.world.entity.EntityEvent;
import net.minecraft.world.entity.Pose;
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
        var dataJson = new JsonObject();

        var entityTypeJson = new JsonObject();
        for (var type : BuiltInRegistries.ENTITY_TYPE) {
            entityTypeJson.addProperty(BuiltInRegistries.ENTITY_TYPE.getKey(type).getPath(), BuiltInRegistries.ENTITY_TYPE.getId(type));
        }
        dataJson.add("entity_type", entityTypeJson);

        var entityEventJson = new JsonObject();
        for (var field : EntityEvent.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                entityEventJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
            }
        }
        dataJson.add("entity_event", entityEventJson);

        var entityAnimationJson = new JsonObject();
        for (var field : ClientboundAnimatePacket.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.canAccess(null) && field.get(null) instanceof Integer i) {
                entityAnimationJson.addProperty(field.getName().toLowerCase(Locale.ROOT), i);
            }
        }
        dataJson.add("entity_animation", entityAnimationJson);

        var villagerTypeJson = new JsonObject();
        for (var type : BuiltInRegistries.VILLAGER_TYPE) {
            villagerTypeJson.addProperty(BuiltInRegistries.VILLAGER_TYPE.getKey(type).getPath(), BuiltInRegistries.VILLAGER_TYPE.getId(type));
        }
        dataJson.add("villager_type", villagerTypeJson);

        var villagerProfessionJson = new JsonObject();
        for (var profession : BuiltInRegistries.VILLAGER_PROFESSION) {
            villagerProfessionJson.addProperty(profession.name(), BuiltInRegistries.VILLAGER_PROFESSION.getId(profession));
        }
        dataJson.add("villager_profession", villagerProfessionJson);

        var catVariantJson = new JsonObject();
        for (var variant : BuiltInRegistries.CAT_VARIANT) {
            catVariantJson.addProperty(BuiltInRegistries.CAT_VARIANT.getKey(variant).getPath(), BuiltInRegistries.CAT_VARIANT.getId(variant));
        }
        dataJson.add("cat_variant", catVariantJson);

        var frogVariantJson = new JsonObject();
        for (var variant : BuiltInRegistries.FROG_VARIANT) {
            frogVariantJson.addProperty(BuiltInRegistries.FROG_VARIANT.getKey(variant).getPath(), BuiltInRegistries.FROG_VARIANT.getId(variant));
        }
        dataJson.add("frog_variant", frogVariantJson);

        var paintingVariantJson = new JsonObject();
        for (var variant : BuiltInRegistries.PAINTING_VARIANT) {
            var variantJson = new JsonObject();
            variantJson.addProperty("id", BuiltInRegistries.PAINTING_VARIANT.getId(variant));
            variantJson.addProperty("width", variant.getWidth());
            variantJson.addProperty("height", variant.getHeight());
            paintingVariantJson.add(BuiltInRegistries.PAINTING_VARIANT.getKey(variant).getPath(), variantJson);
        }
        dataJson.add("painting_variant", paintingVariantJson);

        var directionJson = new JsonObject();
        for (var dir : Direction.values()) {
            directionJson.addProperty(dir.getName(), dir.get3DDataValue());
        }
        dataJson.add("direction", directionJson);

        var poseJson = new JsonObject();
        var poses = Pose.values();
        for (int i = 0; i < poses.length; i++) {
            poseJson.addProperty(poses[i].name().toLowerCase(Locale.ROOT), i);
        }
        dataJson.add("pose", poseJson);

        var particleTypeJson = new JsonObject();
        for (var type : BuiltInRegistries.PARTICLE_TYPE) {
            particleTypeJson.addProperty(BuiltInRegistries.PARTICLE_TYPE.getKey(type).getPath(), BuiltInRegistries.PARTICLE_TYPE.getId(type));
        }
        dataJson.add("particle_type", particleTypeJson);

        return dataJson;
    }
}
