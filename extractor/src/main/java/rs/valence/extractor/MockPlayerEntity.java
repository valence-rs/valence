package rs.valence.extractor;

import com.mojang.authlib.GameProfile;
import net.minecraft.core.BlockPos;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

public class MockPlayerEntity extends Player {
    public static final MockPlayerEntity INSTANCE;

    static {
        INSTANCE = Main.magicallyInstantiate(MockPlayerEntity.class);

        try {
            var entityDataField = Entity.class.getDeclaredField("entityData");
            entityDataField.setAccessible(true);
            entityDataField.set(INSTANCE, new SynchedEntityData(INSTANCE));

            INSTANCE.defineSynchedData();
        } catch (NoSuchFieldException | IllegalAccessException e) {
            throw new RuntimeException(e);
        }
    }

    public MockPlayerEntity(Level level, BlockPos blockPos, float f, GameProfile gameProfile) {
        super(level, blockPos, f, gameProfile);
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
