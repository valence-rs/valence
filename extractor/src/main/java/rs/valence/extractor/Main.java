package rs.valence.extractor;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import io.netty.handler.codec.EncoderException;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.minecraft.nbt.NbtIo;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import rs.valence.extractor.extractors.*;
import sun.reflect.ReflectionFactory;

import java.io.FileWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

public class Main implements ModInitializer {
    public static final String MOD_ID = "valence_extractor";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

    /**
     * Magically creates an instance of a <i>concrete</i> class without calling its constructor.
     */
    public static <T> T magicallyInstantiate(Class<T> clazz) {
        var rf = ReflectionFactory.getReflectionFactory();
        try {
            var objCon = Object.class.getDeclaredConstructor();
            var con = rf.newConstructorForSerialization(clazz, objCon);
            return clazz.cast(con.newInstance());
        } catch (Throwable e) {
            throw new IllegalArgumentException("Failed to magically instantiate " + clazz.getName(), e);
        }
    }

    @Override
    public void onInitialize() {
        LOGGER.info("Starting extractors...");

        var extractors = new Extractor[]{
               new Blocks(),
               new Enchants(),
               new Entities(),
               new Misc(),
               new Items(),
               new Packets(),
               new Sounds(),
               new TranslationKeys(),
        };

        Path outputDirectory;
        try {
            outputDirectory = Files.createDirectories(Paths.get("valence_extractor_output"));
        } catch (IOException e) {
            LOGGER.info("Failed to create output directory.", e);
            return;
        }

        var gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().serializeNulls().create();

        for (var ext : extractors) {
            try {
                var out = outputDirectory.resolve(ext.fileName());
                var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
                gson.toJson(ext.extract(), fileWriter);
                fileWriter.close();
                LOGGER.info("Wrote " + out.toAbsolutePath());
            } catch (Exception e) {
                LOGGER.error("Extractor for \"" + ext.fileName() + "\" failed.", e);
            }
        }

        ServerLifecycleEvents.SERVER_STARTING.register(server -> {
            LOGGER.info("Server starting, Running startup extractors...");
            // TODO: make `Codec` implement `Extractor`
            var codecExtractor = new Codec(server);
            try {
                var out = outputDirectory.resolve(codecExtractor.fileName());
                var compound = codecExtractor.extract();
                // read the compound byte-wise and write it to the file
                try {
                    NbtIo.write(compound, out.toFile());
                } catch (IOException var3) {
                    throw new EncoderException(var3);
                }

                LOGGER.info("Wrote " + out.toAbsolutePath());
            } catch (Exception e) {
                LOGGER.error("Extractor for \"" + codecExtractor.fileName() + "\" failed.", e);
            }

            var startupExtractors = new Extractor[]{
                new Tags(server),
            };

            for (var ext : startupExtractors) {
                try {
                    var out = outputDirectory.resolve(ext.fileName());
                    var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
                    gson.toJson(ext.extract(), fileWriter);
                    fileWriter.close();
                    LOGGER.info("Wrote " + out.toAbsolutePath());
                } catch (Exception e) {
                    LOGGER.error("Extractor for \"" + ext.fileName() + "\" failed.", e);
                }
            }

            LOGGER.info("Done.");
            server.shutdown();
        });
    }

    public interface Extractor {
        String fileName();

        JsonElement extract() throws Exception;
    }

    public record Pair<T, U>(T left, U right) {
    }
}
