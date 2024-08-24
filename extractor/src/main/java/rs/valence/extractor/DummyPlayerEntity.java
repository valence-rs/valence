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
        INSTANCE = Main.magicallyInstantiate(DummyPlayerEntity.class);

        DummyPlayerEntity.INSTANCE.initDataTracker(new DataTracker.Builder(DummyPlayerEntity.INSTANCE));
    }

    public DummyPlayerEntity(final World world, final BlockPos pos, final float yaw, final GameProfile gameProfile, @Nullable final PlayerPublicKey publicKey) {
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
