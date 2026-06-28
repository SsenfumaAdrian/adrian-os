# ADRIAN OS x86_64 Serial Bring-Up Specification v1

## Purpose
Define the first implementation-oriented serial debug path for Axiom on x86_64.

## Initial Target
- COM1-style serial assumptions
- fixed-string output first
- QEMU-friendly bring-up
- panic/debug milestone visibility

## Phases
1. serial backend structure
2. serial initialization placeholder
3. byte write abstraction
4. string write path
5. real port I/O implementation
6. QEMU-visible validation

## Engineering Rule
Keep generic debug APIs separate from architecture-specific serial mechanics.
