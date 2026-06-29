# ADRIAN OS Boot Image to BootContext Bridge v1

## Purpose
Describe the conceptual bridge from boot-image wrapper entry into Axiom BootContext handoff.

## Expected Flow
1. wrapper entry gains control
2. wrapper receives or derives boot-state information
3. wrapper constructs BootContext-compatible data
4. wrapper calls into entry::kernel_entry(&BootContext)

## Important Rule
Temporary experiment assumptions must be documented explicitly and must not be confused with final Halo handoff semantics.
