package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.network.NetworkSide;
import net.minecraft.network.NetworkState;
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

        for (var side : NetworkSide.values()) {
            var sideJson = new JsonObject();

            for (var state : NetworkState.values()) {
                var stateJson = new JsonArray();

                var map = state.getPacketIdToPacketMap(side);

                for (var id : new TreeSet<>(map.keySet())) {
                    var packetJson = new JsonObject();

                    packetJson.addProperty("name", map.get(id.intValue()).getSimpleName());
                    packetJson.addProperty("id", id);

                    stateJson.add(packetJson);
                }

                sideJson.add(state.name().toLowerCase(Locale.ROOT), stateJson);
            }

            packetsJson.add(side.name().toLowerCase(Locale.ROOT), sideJson);
        }

        return packetsJson;
    }
}
