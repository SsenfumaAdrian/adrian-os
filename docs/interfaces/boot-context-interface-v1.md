# ADRIAN OS Boot Context Interface v1

## Purpose
Define the structured Halo -> Axiom boot handoff contract.

## Initial Fields
- magic
- version
- architecture id
- flags
- memory map metadata
- framebuffer placeholder
- active slot metadata placeholder
- debug/developer mode flags

## Rules
- versioned structure
- kernel validates before use
- no raw trust in incoming pointers without validation
