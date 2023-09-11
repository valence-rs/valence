# Valence Command

This plugin manages the command system for a valence server. It is responsible for parsing, storing, managing and
dispatching commands.

#### This plugin manages the following:

- Registering commands to a Command Graph which is used parse commands.
- Receiving commands from the client and turning them into events.
- Parsing commands and dispatching them in the registered executable format.
- Sending the command graph to clients.

See the module level documentation for more information.
