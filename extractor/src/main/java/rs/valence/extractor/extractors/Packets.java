package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.NetworkState;
import net.minecraft.network.listener.PacketListener;
import net.minecraft.network.state.*;
import rs.valence.extractor.Main;

import java.io.IOException;

public class Packets implements Main.Extractor {
    @Override
    public String fileName() {
        return "packets.json";
    }

    @Override
    public JsonElement extract() throws IOException {
        final var packetsJson = new JsonArray();

        Packets.serializeFactory(HandshakeStates.C2S_FACTORY, packetsJson);
        Packets.serializeFactory(QueryStates.C2S_FACTORY, packetsJson);
        Packets.serializeFactory(QueryStates.S2C_FACTORY, packetsJson);
        Packets.serializeFactory(LoginStates.C2S_FACTORY, packetsJson);
        Packets.serializeFactory(LoginStates.S2C_FACTORY, packetsJson);
        Packets.serializeFactory(ConfigurationStates.C2S_FACTORY, packetsJson);
        Packets.serializeFactory(ConfigurationStates.S2C_FACTORY, packetsJson);
        Packets.serializeFactory(PlayStateFactories.C2S, packetsJson);
        Packets.serializeFactory(PlayStateFactories.S2C, packetsJson);

        return packetsJson;
    }

    private static <T extends PacketListener, B extends ByteBuf> void serializeFactory(final NetworkState.Factory<T, B> factory, final JsonArray json) {
        factory.forEachPacketType((type, i) -> {
            final var packetJson = new JsonObject();
            packetJson.addProperty("name", type.id().getPath());
            packetJson.addProperty("phase", factory.phase().getId());
            packetJson.addProperty("side", factory.side().getName());
            packetJson.addProperty("id", i);
            json.add(packetJson);
        });
    }
}
