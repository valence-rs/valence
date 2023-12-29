package rs.valence.extractor;

import com.mojang.authlib.GameProfile;
import net.minecraft.entity.Entity;
import net.minecraft.entity.data.DataTracker;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.network.encryption.PlayerPublicKey;
import net.minecraft.util.math.BlockPos;
import net.minecraft.world.World;
import org.jetbrains.annotations.Nullable;

public class DummyPlayerEntity extends PlayerEntity {
    public static final DummyPlayerEntity INSTANCE;

    static {
        INSTANCE = new DummyPlayerEntity(DummyWorld.INSTANCE, new BlockPos(0, 0, 0), 0, new GameProfile(null, "dummy"),
                null);
        // Main.magicallyInstantiate(DummyPlayerEntity.class);

        try {
            var dataTrackerField = Entity.class.getDeclaredField("dataTracker");
            dataTrackerField.setAccessible(true);
            dataTrackerField.set(INSTANCE, new DataTracker(INSTANCE));

            INSTANCE.initDataTracker();

            INSTANCE.setHealth(20); // idk why player health is set to 1 by default
        } catch (NoSuchFieldException | IllegalAccessException e) {
            throw new RuntimeException(e);
        }
    }

    private DummyPlayerEntity(World world, BlockPos pos, float yaw, GameProfile gameProfile,
            @Nullable PlayerPublicKey publicKey) {
        super(world, pos, yaw, gameProfile);
    }

    @Override
    public boolean isSpectator() {
        return false;
    }

    @Override
    public boolean isCreative() {
        return false;
    }
}
