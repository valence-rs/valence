package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.entity.EntityPose;
import net.minecraft.entity.EntityStatuses;
import net.minecraft.network.packet.s2c.play.EntityAnimationS2CPacket;
import net.minecraft.registry.Registries;
import net.minecraft.util.math.Direction;
import rs.valence.extractor.Main;

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

        var typesJson = new JsonObject();
        for (var type : Registries.ENTITY_TYPE) {
            typesJson.addProperty(Registries.ENTITY_TYPE.getId(type).getPath(), Registries.ENTITY_TYPE.getRawId(type));
        }
        dataJson.add("types", typesJson);

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
        for (var type : Registries.VILLAGER_TYPE) {
            villagerTypesJson.addProperty(Registries.VILLAGER_TYPE.getId(type).getPath(), Registries.VILLAGER_TYPE.getRawId(type));
        }
        dataJson.add("villager_types", villagerTypesJson);

        var villagerProfessionsJson = new JsonObject();
        for (var profession : Registries.VILLAGER_PROFESSION) {
            villagerProfessionsJson.addProperty(profession.id(), Registries.VILLAGER_PROFESSION.getRawId(profession));
        }
        dataJson.add("villager_professions", villagerProfessionsJson);

        var catVariantsJson = new JsonObject();
        for (var variant : Registries.CAT_VARIANT) {
            catVariantsJson.addProperty(Registries.CAT_VARIANT.getId(variant).getPath(), Registries.CAT_VARIANT.getRawId(variant));
        }
        dataJson.add("cat_variants", catVariantsJson);

        var frogVariantsJson = new JsonObject();
        for (var variant : Registries.FROG_VARIANT) {
            frogVariantsJson.addProperty(Registries.FROG_VARIANT.getId(variant).getPath(), Registries.FROG_VARIANT.getRawId(variant));
        }
        dataJson.add("frog_variants", frogVariantsJson);

        var paintingVariantsJson = new JsonObject();
        for (var variant : Registries.PAINTING_VARIANT) {
            var variantJson = new JsonObject();
            variantJson.addProperty("id", Registries.PAINTING_VARIANT.getRawId(variant));
            variantJson.addProperty("width", variant.getWidth());
            variantJson.addProperty("height", variant.getHeight());
            paintingVariantsJson.add(Registries.PAINTING_VARIANT.getId(variant).getPath(), variantJson);
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
        for (var type : Registries.PARTICLE_TYPE) {
            particleTypesJson.addProperty(Registries.PARTICLE_TYPE.getId(type).getPath(), Registries.PARTICLE_TYPE.getRawId(type));
        }
        dataJson.add("particle_types", particleTypesJson);

        return dataJson;
    }
}
