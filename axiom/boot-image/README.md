# Axiom Boot Image

The Axiom Boot Image crate is the future wrapper and artifact path for bootable ADRIAN OS kernel experiments.

## Purpose
This crate exists to separate:
- kernel core logic
from
- boot artifact concerns

## Intended Responsibilities
- future external entry symbol ownership
- future target-specific build evolution
- future linker integration
- future boot artifact packaging
- future QEMU boot experiment support
- future Halo handoff experiment alignment

## Non-Responsibilities
This crate should not:
- become a second kernel
- duplicate Axiom subsystem logic
- absorb unrelated runtime internals
- replace Halo as a trust-launch system

## Current State
This crate is still a scaffold and not yet a true bootable kernel artifact.
