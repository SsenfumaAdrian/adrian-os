# ADRIAN OS Synthetic BootContext Boundary v1

## Purpose
Clarify where synthetic BootContext may originate and what boundaries it must respect.

## Boundary Rule
Synthetic BootContext belongs on the wrapper or experiment side, not as hidden kernel-core truth.

## Stable Semantic Expectations
- magic identifies the structure
- version identifies the schema
- architecture identifies the target architecture
- flags describe handoff state
- memory map info remains memory-map-related
- framebuffer info remains display-bootstrap-related
