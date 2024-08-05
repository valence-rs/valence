package rs.valence.extractor.extractors;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.minecraft.data.DataOutput;
import net.minecraft.data.report.PacketReportProvider;
import net.minecraft.network.NetworkSide;
import net.minecraft.network.NetworkState;
import net.minecraft.network.state.*;
import rs.valence.extractor.Main;

import java.lang.reflect.Method;
import java.util.Arrays;
import java.util.Locale;
import java.util.TreeSet;
import java.util.stream.Stream;

public class Packets implements Main.Extractor {
    public static final JsonArray PACKETS = new JsonArray();
    @Override
    public String fileName() {
        return "packets.json";
    }

    @Override
    public JsonElement extract() {
//        var packetsReportProvider = Main.magicallyInstantiate(PacketReportProvider.class);
//        try {
//            // Obtain the private method `toJson`
//            Method toJsonMethod = PacketReportProvider.class.getDeclaredMethod("toJson");
//            // Make the method accessible
//            toJsonMethod.setAccessible(true);
//            // Invoke the method and get the result
//            JsonObject packetsJson = (JsonObject) toJsonMethod.invoke(packetsReportProvider);
//            return packetsJson;
//        } catch (Exception e) {
//            throw new RuntimeException("Failed to invoke toJson method", e);
//        }

        Stream.of(
                HandshakeStates.C2S_FACTORY,
                QueryStates.S2C_FACTORY,
                QueryStates.C2S_FACTORY,
                LoginStates.S2C_FACTORY,
                LoginStates.C2S_FACTORY,
                ConfigurationStates.S2C_FACTORY,
                ConfigurationStates.C2S_FACTORY,
                PlayStateFactories.S2C,
                PlayStateFactories.C2S
        ).forEach(state -> {
            state.forEachPacketType((packetType, packet) -> {
//                Main.LOGGER.info(Arrays.toString(packetType.getClass().getTypeParameters()) + "   " + packet);
            });
        });

        return null;

    }
}
