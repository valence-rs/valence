# Valence Extractor

This is a Fabric mod for Minecraft that extracts data about different things in Minecraft, like blocks, packets, etc. All the extracted data is stored in the sibling `extracted` folder.

## How to use

Here's how to regenerate the contents of `extracted`.

From this directory, run the following

```sh
./gradlew runServer
```

This will run the extractor and immediately exit, outputting the files that are listed in the logs.

Next, run `copy_extractor_output.sh`. This copies the files to `extracted` so that they can be comitted.

```sh
./copy_extractor_output.sh
```

## How to update valence to a new version of Minecraft

The general process should go something like this:
1. Update `gradle.properties` to the new version of Minecraft using https://fabricmc.net/develop
2. Update `src/main/resources/fabric.mod.json` to reference new version of Minecraft
3. Update `PROTOCOL_VERSION` and `MINECRAFT_VERSION` constants in `valence_core/src/lib.rs`
4. Attempt to run `./gradlew runServer` and fix any errors that come up
5. Run `./copy_extractor_output.sh`
6. In `*.toml`s, replace all strings of the old mc version with the new mc version
7. Update the download URL in `tools/download_vanilla_server`.
8. Try all the examples. If they work, you're probably done.


If you need to update gradle, running this will automatically update the wrapper to the specified version, and update `gradle/gradle-wrapper.properties`.
```sh
./gradlew wrapper --gradle-version VERSION
```

You may also need to update the fabric mappings in the mod.
```sh
./gradlew migrateMappings --mappings "VERSION"
```

## Contributing

Run `./gradlew genSources` to generate Minecraft Java source files for your IDE.
