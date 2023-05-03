# valence_chat

Provides support for cryptographically verified chat messaging on the server.

This crate contains the secure chat plugin as well as chat types and the chat type registry. Minecraft's default chat types are added to the registry by default. Chat types contain information about how chat is styled, such as the chat color.

### **NOTE:**
- Modifying the chat type registry after the server has started can
break invariants within instances and clients! Make sure there are no
instances or clients spawned before mutating.