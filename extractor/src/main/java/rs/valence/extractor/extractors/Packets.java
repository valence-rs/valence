package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.network.ConnectionProtocol;
import net.minecraft.network.protocol.PacketFlow;
import rs.valence.extractor.Main;

import java.util.Locale;
import java.util.TreeSet;

public class Packets implements Main.Extractor {
    @Override
    public String fileName() {
        return "packets.json";
    }

    @Override
    public JsonElement extract() {
        var packetsJson = new JsonObject();

        for (var flow : PacketFlow.values()) {
            var flowJson = new JsonObject();

            for (var proto : ConnectionProtocol.values()) {
                var protoJson = new JsonArray();

                var map = proto.getPacketsByIds(flow);

                for (var id : new TreeSet<>(map.keySet())) {
                    var packetJson = new JsonObject();

                    packetJson.addProperty("name", map.get(id.intValue()).getSimpleName());
                    packetJson.addProperty("id", id);

                    protoJson.add(packetJson);
                }

                flowJson.add(proto.name().toLowerCase(Locale.ROOT), protoJson);
            }

            packetsJson.add(flow.name().toLowerCase(Locale.ROOT), flowJson);
        }

        return packetsJson;
    }
}
