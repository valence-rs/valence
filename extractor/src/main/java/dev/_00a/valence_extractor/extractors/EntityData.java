package dev._00a.valence_extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import dev._00a.valence_extractor.Main;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.network.packet.s2c.play.EntityAnimationS2CPacket;
import net.minecraft.util.math.Direction;
import net.minecraft.util.registry.Registry;

import java.lang.reflect.Modifier;
import java.util.Locale;

public class EntityData implements Main.Extractor {
    @Override
    public String fileName() {
        return "entity_data.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        var dataJson = new JsonObject();

        var statusesJson = new JsonObject();
        for (var field : EntityStatuses.class.getDeclaredFields()) {
            if (field.canAccess(null) && field.get(null) instanceof Byte code) {
                if (field.getName().equals("field_30030")) {
                    statusesJson.addProperty("stop_attack", code);
                } else {
                    statusesJson.addProperty(field.getName().toLowerCase(Locale.ROOT), code);
                }
            }
        }
        dataJson.add("statuses", statusesJson);

        var animationsJson = new JsonObject();
        for (var field : EntityAnimationS2CPacket.class.getDeclaredFields()) {
            field.setAccessible(true);
            if (Modifier.isStatic(field.getModifiers()) && field.canAccess(null) && field.get(null) instanceof Integer i) {
                animationsJson.addProperty(field.getName().toLowerCase(Locale.ROOT), i);
            }
        }
        dataJson.add("animations", animationsJson);

        var villagerTypesJson = new JsonObject();
        for (var type : Registry.VILLAGER_TYPE) {
            villagerTypesJson.addProperty(Registry.VILLAGER_TYPE.getId(type).getPath(), Registry.VILLAGER_TYPE.getRawId(type));
        }
        dataJson.add("villager_types", villagerTypesJson);

        var villagerProfessionsJson = new JsonObject();
        for (var profession : Registry.VILLAGER_PROFESSION) {
            villagerProfessionsJson.addProperty(profession.id(), Registry.VILLAGER_PROFESSION.getRawId(profession));
        }
        dataJson.add("villager_professions", villagerProfessionsJson);

        var catVariantsJson = new JsonObject();
        for (var variant : Registry.CAT_VARIANT) {
            catVariantsJson.addProperty(Registry.CAT_VARIANT.getId(variant).getPath(), Registry.CAT_VARIANT.getRawId(variant));
        }
        dataJson.add("cat_variants", catVariantsJson);

        var frogVariantsJson = new JsonObject();
        for (var variant : Registry.FROG_VARIANT) {
            frogVariantsJson.addProperty(Registry.FROG_VARIANT.getId(variant).getPath(), Registry.FROG_VARIANT.getRawId(variant));
        }
        dataJson.add("frog_variants", frogVariantsJson);

        var paintingVariantsJson = new JsonObject();
        for (var variant : Registry.PAINTING_VARIANT) {
            var variantJson = new JsonObject();
            variantJson.addProperty("id", Registry.PAINTING_VARIANT.getRawId(variant));
            variantJson.addProperty("width", variant.getWidth());
            variantJson.addProperty("height", variant.getHeight());
            paintingVariantsJson.add(Registry.PAINTING_VARIANT.getId(variant).getPath(), variantJson);
        }
        dataJson.add("painting_variants", paintingVariantsJson);

        var facingJson = new JsonObject();
        for (var dir : Direction.values()) {
            facingJson.addProperty(dir.getName(), dir.getId());
        }
        dataJson.add("facing", facingJson);

        var posesJson = new JsonObject();
        var poses = EntityPose.values();
        for (int i = 0; i < poses.length; i++) {
            posesJson.addProperty(poses[i].name().toLowerCase(Locale.ROOT), i);
        }
        dataJson.add("poses", posesJson);

        var particleTypesJson = new JsonObject();
        for (var type : Registry.PARTICLE_TYPE) {
            particleTypesJson.addProperty(Registry.PARTICLE_TYPE.getId(type).getPath(), Registry.PARTICLE_TYPE.getRawId(type));
        }
        dataJson.add("particle_types", particleTypesJson);

        return dataJson;
    }
}
