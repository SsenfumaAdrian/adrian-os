# ADRIAN OS Kernel Entry Interface v1

## Purpose
Define the internal Axiom entry boundary after Halo transfers control.

## Planned Responsibilities
- receive BootContext
- validate boot handoff
- initialize architecture-specific early state
- transition into generic kernel init flow

## Conceptual Layers
1. bootloader handoff
2. kernel entry boundary
3. early arch initialization
4. early subsystem initialization
5. kernel idle/init transition

## Entry Design Rule
The entry interface should remain minimal, deterministic, and easy to debug.
