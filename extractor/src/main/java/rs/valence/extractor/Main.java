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
    public static final Logger LOGGER = LoggerFactory.getLogger(Main.MOD_ID);

    /**
     * Magically creates an instance of a <i>concrete</i> class without calling its constructor.
     */
    public static <T> T magicallyInstantiate(final Class<T> clazz) {
        final var rf = ReflectionFactory.getReflectionFactory();
        try {
            final var objCon = Object.class.getDeclaredConstructor();
            final var con = rf.newConstructorForSerialization(clazz, objCon);
            return clazz.cast(con.newInstance());
        } catch (final Throwable e) {
            throw new IllegalArgumentException("Failed to magically instantiate " + clazz.getName(), e);
        }
    }

    @Override
    public void onInitialize() {
        Main.LOGGER.info("Starting extractors...");

        final var extractors = new Extractor[]{
               new Attributes(),
               new Blocks(),
               new Effects(),
               new Misc(),
               new Items(),
               new Packets(),
               new Sounds(),
               new TranslationKeys(),
        };

        final Path outputDirectory;
        try {
            outputDirectory = Files.createDirectories(Paths.get("valence_extractor_output"));
        } catch (final IOException e) {
            Main.LOGGER.info("Failed to create output directory.", e);
            return;
        }

        final var gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().serializeNulls().create();

        for (final var ext : extractors) {
            try {
                final var out = outputDirectory.resolve(ext.fileName());
                final var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
                gson.toJson(ext.extract(), fileWriter);
                fileWriter.close();
                Main.LOGGER.info("Wrote " + out.toAbsolutePath());
            } catch (final Exception e) {
                Main.LOGGER.error("Extractor for \"" + ext.fileName() + "\" failed.", e);
            }
        }

        ServerLifecycleEvents.SERVER_STARTED.register(server -> {
            Main.LOGGER.info("Server starting, Running startup extractors...");
            // TODO: make `Codec` implement `Extractor`
            // TODO: the way to get Codex has changed, this is not working anymore
            final var packetRegistryExtractor = new PacketRegistries(server);
            try {
                final var out = outputDirectory.resolve(packetRegistryExtractor.fileName());
                final var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
                gson.toJson(packetRegistryExtractor.extract(), fileWriter);
                fileWriter.close();

                Main.LOGGER.info("Wrote " + out.toAbsolutePath());
            } catch (final Exception e) {
                Main.LOGGER.error("Extractor for \"" + packetRegistryExtractor.fileName() + "\" failed.", e);
            }

            final var startupExtractors = new Extractor[]{
                new Tags(server),
                new Paintings(server),
                new Enchants(server),
                new Entities(server),
            };

            for (final var ext : startupExtractors) {
                try {
                    final var out = outputDirectory.resolve(ext.fileName());
                    final var fileWriter = new FileWriter(out.toFile(), StandardCharsets.UTF_8);
                    gson.toJson(ext.extract(), fileWriter);
                    fileWriter.close();
                    Main.LOGGER.info("Wrote " + out.toAbsolutePath());
                } catch (final Exception e) {
                    Main.LOGGER.error("Extractor for \"" + ext.fileName() + "\" failed.", e);
                }
            }

            Main.LOGGER.info("Done.");
            server.stop(false);
        });
    }

    public interface Extractor {
        String fileName();

        JsonElement extract() throws Exception;
    }

    public record Pair<T, U>(T left, U right) {
    }
}
