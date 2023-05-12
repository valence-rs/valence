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

## Contributing

Run `./gradlew genSources` to generate Minecraft Java source files for your IDE.
