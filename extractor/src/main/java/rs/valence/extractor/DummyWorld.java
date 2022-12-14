package rs.valence.extractor;

import net.minecraft.block.Block;
import net.minecraft.block.BlockState;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.fluid.Fluid;
import net.minecraft.item.map.MapState;
import net.minecraft.recipe.RecipeManager;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.RegistryKey;
import net.minecraft.registry.entry.RegistryEntry;
import net.minecraft.resource.featuretoggle.FeatureFlags;
import net.minecraft.resource.featuretoggle.FeatureSet;
import net.minecraft.scoreboard.Scoreboard;
import net.minecraft.sound.SoundCategory;
import net.minecraft.sound.SoundEvent;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.Direction;
import net.minecraft.util.math.Vec3d;
import net.minecraft.util.math.random.Random;
import net.minecraft.util.profiler.Profiler;
import net.minecraft.world.Difficulty;
import net.minecraft.world.GameRules;
import net.minecraft.world.MutableWorldProperties;
import net.minecraft.world.World;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.chunk.ChunkManager;
import net.minecraft.world.dimension.DimensionType;
import net.minecraft.world.entity.EntityLookup;
import net.minecraft.world.event.GameEvent;
import net.minecraft.world.tick.QueryableTickScheduler;
import org.jetbrains.annotations.Nullable;

import java.util.List;
import java.util.function.Supplier;

public class DummyWorld extends World {

    public static final DummyWorld INSTANCE;

    static {
        INSTANCE = Main.magicallyInstantiate(DummyWorld.class);

        try {
            var randomField = World.class.getDeclaredField("random");
            randomField.setAccessible(true);
            randomField.set(INSTANCE, Random.create());

            var propertiesField = World.class.getDeclaredField("properties");
            propertiesField.setAccessible(true);
            propertiesField.set(INSTANCE, new DummyMutableWorldProperties());

        } catch (NoSuchFieldException | IllegalAccessException e) {
            throw new RuntimeException(e);
        }
    }

    private DummyWorld(MutableWorldProperties properties, RegistryKey<World> registryRef, RegistryEntry<DimensionType> dimension, Supplier<Profiler> profiler, boolean isClient, boolean debugWorld, long seed, int maxChainedNeighborUpdates) {
        super(properties, registryRef, dimension, profiler, isClient, debugWorld, seed, maxChainedNeighborUpdates);
    }

    @Override
    public void updateListeners(BlockPos pos, BlockState oldState, BlockState newState, int flags) {

    }

    @Override
    public void playSound(@Nullable PlayerEntity except, double x, double y, double z, RegistryEntry<SoundEvent> sound, SoundCategory category, float volume, float pitch, long seed) {

    }

    @Override
    public void playSound(@Nullable PlayerEntity except, double x, double y, double z, SoundEvent sound, SoundCategory category, float volume, float pitch, long seed) {

    }

    @Override
    public void playSoundFromEntity(@Nullable PlayerEntity except, Entity entity, RegistryEntry<SoundEvent> sound, SoundCategory category, float volume, float pitch, long seed) {

    }

    @Override
    public String asString() {
        return "";
    }

    @Nullable
    @Override
    public Entity getEntityById(int id) {
        return null;
    }

    @Nullable
    @Override
    public MapState getMapState(String id) {
        return null;
    }

    @Override
    public void putMapState(String id, MapState state) {

    }

    @Override
    public int getNextMapId() {
        return 0;
    }

    @Override
    public void setBlockBreakingInfo(int entityId, BlockPos pos, int progress) {

    }

    @Override
    public Scoreboard getScoreboard() {
        return new Scoreboard();
    }

    @Override
    public RecipeManager getRecipeManager() {
        return new RecipeManager();
    }

    @Override
    protected EntityLookup<Entity> getEntityLookup() {
        return null;
    }

    @Override
    public QueryableTickScheduler<Block> getBlockTickScheduler() {
        return null;
    }

    @Override
    public QueryableTickScheduler<Fluid> getFluidTickScheduler() {
        return null;
    }

    @Override
    public ChunkManager getChunkManager() {
        return null;
    }

    @Override
    public void syncWorldEvent(@Nullable PlayerEntity player, int eventId, BlockPos pos, int data) {

    }

    @Override
    public void emitGameEvent(GameEvent event, Vec3d emitterPos, GameEvent.Emitter emitter) {

    }

    @Override
    public DynamicRegistryManager getRegistryManager() {
        return null;
    }

    @Override
    public FeatureSet getEnabledFeatures() {
        return FeatureSet.of(FeatureFlags.VANILLA, FeatureFlags.BUNDLE, FeatureFlags.UPDATE_1_20);
    }

    @Override
    public float getBrightness(Direction direction, boolean shaded) {
        return 0;
    }

    @Override
    public List<? extends PlayerEntity> getPlayers() {
        return List.of();
    }

    @Override
    public RegistryEntry<Biome> getGeneratorStoredBiome(int biomeX, int biomeY, int biomeZ) {
        return null;
    }

    private static class DummyMutableWorldProperties implements MutableWorldProperties {

        @Override
        public int getSpawnX() {
            return 0;
        }

        @Override
        public void setSpawnX(int spawnX) {

        }

        @Override
        public int getSpawnY() {
            return 0;
        }

        @Override
        public void setSpawnY(int spawnY) {

        }

        @Override
        public int getSpawnZ() {
            return 0;
        }

        @Override
        public void setSpawnZ(int spawnZ) {

        }

        @Override
        public float getSpawnAngle() {
            return 0;
        }

        @Override
        public void setSpawnAngle(float spawnAngle) {

        }

        @Override
        public long getTime() {
            return 0;
        }

        @Override
        public long getTimeOfDay() {
            return 0;
        }

        @Override
        public boolean isThundering() {
            return false;
        }

        @Override
        public boolean isRaining() {
            return false;
        }

        @Override
        public void setRaining(boolean raining) {

        }

        @Override
        public boolean isHardcore() {
            return false;
        }

        @Override
        public GameRules getGameRules() {
            return null;
        }

        @Override
        public Difficulty getDifficulty() {
            return null;
        }

        @Override
        public boolean isDifficultyLocked() {
            return false;
        }
    }
}
