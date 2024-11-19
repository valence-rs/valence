package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.Codec;
import com.mojang.serialization.JsonOps;
import net.minecraft.entity.decoration.painting.PaintingVariant;
import net.minecraft.recipe.RecipeSerializer;
import net.minecraft.recipe.RecipeType;
import net.minecraft.recipe.ServerRecipeManager;
import net.minecraft.recipe.display.RecipeDisplay;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.registry.RegistryOps;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

import java.util.Optional;

public class Recipe implements Main.Extractor {

    private final DynamicRegistryManager.Immutable registryManager;
    private final ServerRecipeManager recipeManager;

    public Recipe(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
        this.recipeManager = server.getRecipeManager();
    }

    @Override
    public String fileName() {
        return "recipes.json";
    }

    @Override
    public JsonElement extract() throws Exception {
        Codec<net.minecraft.recipe.Recipe<?>> codec = Registries.RECIPE_SERIALIZER.getCodec().dispatch(net.minecraft.recipe.Recipe::getSerializer, RecipeSerializer::codec);
        JsonObject json = new JsonObject();
        recipeManager
                .values()
            .forEach(entry -> {
//                JsonObject inner = new JsonObject();
//                inner.addProperty("type", recipe.getType().toString());
//                var list = new JsonArray();
//                recipe.getDisplays().forEach(layout -> {
//                    list.add(RecipeDisplay.CODEC.encodeStart(JsonOps.INSTANCE, layout).getOrThrow());
//                });
//                inner.add("displays", list);
//
                json.add(
                    entry.id().getValue().getPath(),
                    codec.encodeStart(JsonOps.INSTANCE, entry.value()).getOrThrow()
                );
            });

        return json;
    }
}
