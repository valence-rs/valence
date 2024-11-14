package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import io.netty.buffer.ByteBuf;
import java.io.IOException;
import net.minecraft.network.NetworkState;
import net.minecraft.network.listener.PacketListener;
import net.minecraft.network.state.*;
import rs.valence.extractor.Main;

public class Packets implements Main.Extractor {

    @Override
    public String fileName() {
        return "packets.json";
    }

    @Override
    public JsonElement extract() throws IOException {
        var packetsJson = new JsonArray();

        serializeFactory(HandshakeStates.C2S_FACTORY, packetsJson);
        serializeFactory(QueryStates.C2S_FACTORY, packetsJson);
        serializeFactory(QueryStates.S2C_FACTORY, packetsJson);
        serializeFactory(LoginStates.C2S_FACTORY, packetsJson);
        serializeFactory(LoginStates.S2C_FACTORY, packetsJson);
        serializeFactory(ConfigurationStates.C2S_FACTORY, packetsJson);
        serializeFactory(ConfigurationStates.S2C_FACTORY, packetsJson);
        serializeFactory(PlayStateFactories.C2S, packetsJson);
        serializeFactory(PlayStateFactories.S2C, packetsJson);

        return packetsJson;
    }

    private static <
        T extends PacketListener, B extends ByteBuf
    > void serializeFactory(
        NetworkState.Factory<T, B> factory,
        JsonArray json
    ) {
        factory.forEachPacketType((type, i) -> {
            var packetJson = new JsonObject();
            packetJson.addProperty("name", type.id().getPath());
            packetJson.addProperty("phase", factory.phase().getId());
            packetJson.addProperty("side", factory.side().getName());
            packetJson.addProperty("id", i);
            json.add(packetJson);
        });
    }
}
