# Valence Extractor

This is a Fabric mod for Minecraft that extracts data about different things in Minecraft, like blocks, packets, etc. All the extracted data is stored in the sibling `extracted` folder.

### How to use

Here's how to regenerate the contents of `extracted`.

From this directory, run the following

```sh
./gradlew runServer
```

This will run the extractor and immediately exit, outputting the files that are listed in the logs. These need to be manually moved to `extracted` to be committed.
