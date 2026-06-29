# ADRIAN OS BootContext Bridge Plan v1

## Purpose
Define how future boot-image wrapper logic should bridge into Axiom's BootContext-based internal entry model.

## Core Principle
The wrapper should prepare or adapt handoff state into BootContext form and then transfer control into kernel core.

## Wrapper Responsibilities
- adapt temporary experiment or loader-facing state
- construct BootContext-compatible handoff data
- document deviations from final Halo-integrated behavior

## Kernel Responsibilities
- validate BootContext
- proceed with internal early initialization
- remain authoritative for runtime progression
