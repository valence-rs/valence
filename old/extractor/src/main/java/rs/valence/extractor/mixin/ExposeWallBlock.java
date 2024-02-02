package rs.valence.extractor.mixin;

import net.minecraft.block.Block;
import net.minecraft.item.VerticallyAttachableBlockItem;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(VerticallyAttachableBlockItem.class)
public interface ExposeWallBlock {
    @Accessor
    Block getWallBlock();
}
