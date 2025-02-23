package rs.valence.extractor.extractors;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.Codec;
import com.mojang.serialization.JsonOps;
import net.minecraft.recipe.RecipeSerializer;
import net.minecraft.recipe.ServerRecipeManager;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryKeys;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

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
        Codec<net.minecraft.recipe.Recipe<?>> codec =
            Registries.RECIPE_SERIALIZER.getCodec()
                .dispatch(
                    net.minecraft.recipe.Recipe::getSerializer,
                    RecipeSerializer::codec
                );
        JsonObject json = new JsonObject();

        JsonObject recipesJson = new JsonObject();
        recipeManager
            .values()
            .forEach(entry -> {
                recipesJson.add(
                    entry.id().getValue().getPath(),
                    codec
                        .encodeStart(JsonOps.INSTANCE, entry.value())
                        .getOrThrow()
                );
            });

        JsonObject displaysJson = new JsonObject();
        var displays = registryManager.getOrThrow(RegistryKeys.RECIPE_DISPLAY);
        var displayCodec = displays.getCodec();

        displays
            .stream()
            .forEach(display -> {
                displaysJson.addProperty(
                    displayCodec
                        .encodeStart(JsonOps.INSTANCE, display)
                        .getOrThrow()
                        .getAsString(),
                    displays.getRawId(display)
                );
            });

        JsonObject bookCategoryJson = new JsonObject();
        var bookCategory = registryManager.getOrThrow(
            RegistryKeys.RECIPE_BOOK_CATEGORY
        );
        var bookCategoryCodec = bookCategory.getEntryCodec();

        bookCategory
            .streamEntries()
            .forEach(entry -> {
                bookCategoryJson.addProperty(
                    bookCategoryCodec
                        .encodeStart(JsonOps.INSTANCE, entry)
                        .getOrThrow()
                        .getAsString(),
                    bookCategory.getRawId(entry.value())
                );
            });

        json.add("recipes", recipesJson);
        json.add("displays", displaysJson);
        json.add("book_categories", bookCategoryJson);

        return json;
    }
}
