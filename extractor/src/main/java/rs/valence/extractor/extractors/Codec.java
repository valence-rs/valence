package rs.valence.extractor.extractors;

import io.netty.handler.codec.EncoderException;
import net.minecraft.nbt.NbtCompound;
import net.minecraft.nbt.NbtElement;
import net.minecraft.nbt.NbtOps;
import net.minecraft.registry.DynamicRegistryManager;
import net.minecraft.registry.Registries;
import net.minecraft.registry.RegistryOps;
import net.minecraft.registry.SerializableRegistries;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.Util;

public class Codec {

    private static final RegistryOps<NbtElement> REGISTRY_OPS= RegistryOps.of(NbtOps.INSTANCE, DynamicRegistryManager.of(Registries.REGISTRIES));
        private final DynamicRegistryManager.Immutable registryManager;

    public Codec(MinecraftServer server) {
        this.registryManager = server.getRegistryManager();
    }

    public String fileName() {
        return "registry_codec.dat";
    }

    public NbtCompound extract() {
//        com.mojang.serialization.Codec<DynamicRegistryManager> codec = SerializableRegistries.CODEC;
//        //DynamicRegistryManager.get(net.minecraft.registry.RegistryKey<? extends net.minecraft.registry.Registry<? extends E>>) method.
//
//        NbtElement nbtElement = Util.getResult(codec.encodeStart(REGISTRY_OPS, registryManager), (error) -> new EncoderException("Failed to encode: " + error + " " + registryManager));
//        return (NbtCompound) nbtElement;
        return null;
    }
}
