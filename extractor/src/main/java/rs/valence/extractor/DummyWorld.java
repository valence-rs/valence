package rs.valence.extractor;

import net.minecraft.block.Block;
import net.minecraft.block.BlockState;
import net.minecraft.component.type.MapIdComponent;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.fluid.Fluid;
import net.minecraft.item.map.MapState;
import net.minecraft.recipe.BrewingRecipeRegistry;
import net.minecraft.recipe.RecipeManager;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
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
import net.minecraft.world.tick.TickManager;
import org.jetbrains.annotations.Nullable;
import java.util.List;
import java.util.function.Supplier;

public class DummyWorld extends World {

    public static final DummyWorld INSTANCE;

    static {
        INSTANCE = Main.magicallyInstantiate(DummyWorld.class);

        try {
            final var randomField = World.class.getDeclaredField("random");
            randomField.setAccessible(true);
            randomField.set(DummyWorld.INSTANCE, Random.create());

            final var propertiesField = World.class.getDeclaredField("properties");
            propertiesField.setAccessible(true);
            propertiesField.set(DummyWorld.INSTANCE, new DummyMutableWorldProperties());

        } catch (final NoSuchFieldException | IllegalAccessException e) {
            throw new RuntimeException(e);
        }
    }

    private DummyWorld(final MutableWorldProperties properties, final RegistryKey<World> registryRef, final DynamicRegistryManager registryManager, final RegistryEntry<DimensionType> dimension, final Supplier<Profiler> profiler, final boolean isClient, final boolean debugWorld, final long seed, final int maxChainedNeighborUpdates) {
        super(properties, registryRef, registryManager, dimension, profiler, isClient, debugWorld, seed, maxChainedNeighborUpdates);
    }

    @Override
    public void updateListeners(final BlockPos pos, final BlockState oldState, final BlockState newState, final int flags) {

    }

    @Override
    public void playSound(@Nullable final PlayerEntity except, final double x, final double y, final double z, final RegistryEntry<SoundEvent> sound, final SoundCategory category, final float volume, final float pitch, final long seed) {

    }

    @Override
    public void playSound(@Nullable final PlayerEntity except, final double x, final double y, final double z, final SoundEvent sound, final SoundCategory category, final float volume, final float pitch, final long seed) {

    }

    @Override
    public void playSoundFromEntity(@Nullable final PlayerEntity except, final Entity entity, final RegistryEntry<SoundEvent> sound, final SoundCategory category, final float volume, final float pitch, final long seed) {

    }

    @Override
    public String asString() {
        return "";
    }

    @Nullable
    @Override
    public Entity getEntityById(final int id) {
        return null;
    }

    @Override
    public TickManager getTickManager() {
        return null;
    }

    @Nullable
    @Override
    public MapState getMapState(final MapIdComponent id) {
        return null;
    }

    @Override
    public void putMapState(final MapIdComponent id, final MapState state) {

    }

    @Override
    public MapIdComponent increaseAndGetMapId() {
        return null;
    }

    @Override
    public void setBlockBreakingInfo(final int entityId, final BlockPos pos, final int progress) {

    }

    @Override
    public Scoreboard getScoreboard() {
        return new Scoreboard();
    }

    @Override
    public RecipeManager getRecipeManager() {
        return null;
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
    public void syncWorldEvent(@Nullable final PlayerEntity player, final int eventId, final BlockPos pos, final int data) {

    }

    @Override
    public void emitGameEvent(final RegistryEntry<GameEvent> event, final Vec3d emitterPos, final GameEvent.Emitter emitter) {

    }


    @Override
    public DynamicRegistryManager getRegistryManager() {
        return DynamicRegistryManager.of(Registries.REGISTRIES);
    }

    @Override
    public BrewingRecipeRegistry getBrewingRecipeRegistry() {
        return null;
    }

    @Override
    public FeatureSet getEnabledFeatures() {
        return FeatureSet.of(FeatureFlags.VANILLA, FeatureFlags.BUNDLE);
    }

    @Override
    public float getBrightness(final Direction direction, final boolean shaded) {
        return 0;
    }

    @Override
    public List<? extends PlayerEntity> getPlayers() {
        return List.of();
    }

    @Override
    public RegistryEntry<Biome> getGeneratorStoredBiome(final int biomeX, final int biomeY, final int biomeZ) {
        return null;
    }

    private static class DummyMutableWorldProperties implements MutableWorldProperties {


        @Override
        public BlockPos getSpawnPos() {
            return null;
        }

        @Override
        public float getSpawnAngle() {
            return 0;
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
        public void setRaining(final boolean raining) {

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

        @Override
        public void setSpawnPos(final BlockPos pos, final float angle) {

        }
    }
}
