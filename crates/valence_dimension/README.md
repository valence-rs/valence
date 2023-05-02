# valence_dimension

Contains dimension types and the dimension type registry. Minecraft's default dimensions are added to the registry by default.

### **NOTE:**
- Modifying the dimension type registry after the server has started can
break invariants within instances and clients! Make sure there are no
instances or clients spawned before mutating.
