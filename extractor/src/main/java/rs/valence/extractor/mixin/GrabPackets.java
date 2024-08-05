package rs.valence.extractor.mixin;

import com.google.common.reflect.TypeToken;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.NetworkStateBuilder;
import net.minecraft.network.codec.PacketCodec;
import net.minecraft.network.listener.PacketListener;
import net.minecraft.network.packet.Packet;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.gen.Accessor;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import rs.valence.extractor.Main;
import rs.valence.extractor.extractors.Packets;

import java.util.List;

@Mixin(NetworkStateBuilder.class)
abstract class GrabPackets <T extends PacketListener, B extends ByteBuf>  {

    @Inject(method = "add", at = @At("HEAD"))
    protected <P extends Packet<?>> void onAdd(net.minecraft.network.packet.PacketType<P> id, PacketCodec<? super B, P> codec, CallbackInfoReturnable ci) {
        try {
//            TypeToken<P> typeToken = new TypeToken<>(getClass()) {
//            };
//            System.out.println(typeToken.getRawType().getSimpleName());
            System.out.println(id.toString());
        } catch (NullPointerException e) {
            e.printStackTrace();
        }

    }
}
