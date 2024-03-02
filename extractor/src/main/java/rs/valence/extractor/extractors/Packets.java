package rs.valence.extractor.extractors;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.minecraft.network.NetworkSide;
import net.minecraft.network.NetworkState;
import net.minecraft.server.MinecraftServer;
import rs.valence.extractor.Main;

import java.io.DataOutput;
import java.io.IOException;
import java.util.Locale;
import java.util.TreeSet;

public class Packets implements Main.Extractor {
    @Override
    public String fileName() {
        return "packets.json";
    }

    @Override
    public void extract(MinecraftServer server, DataOutput output, Gson gson) throws IOException {
        var packetsJson = new JsonArray();

        for (var state : NetworkState.values()) {
            for (var side : NetworkSide.values()) {
                var map = state.getPacketIdToPacketMap(side);

                for (var id : new TreeSet<>(map.keySet())) {
                    var packetJson = new JsonObject();

                    packetJson.addProperty("name", map.get(id.intValue()).getSimpleName());
                    packetJson.addProperty("state", state.name().toLowerCase(Locale.ROOT));
                    packetJson.addProperty("side", side.name().toLowerCase(Locale.ROOT));
                    packetJson.addProperty("id", id);

                    packetsJson.add(packetJson);
                }
            }
        }

        Main.writeJson(output, gson, packetsJson);
    }
}
