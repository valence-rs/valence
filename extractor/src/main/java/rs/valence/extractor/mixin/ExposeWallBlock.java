package rs.valence.extractor.mixin;

import net.minecraft.block.Block;
import net.minecraft.item.WallStandingBlockItem;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(WallStandingBlockItem.class)
public interface ExposeWallBlock {
    @Accessor
    Block getWallBlock();
}
