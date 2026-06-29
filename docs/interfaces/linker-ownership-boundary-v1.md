# ADRIAN OS Linker Ownership Boundary v1

## Purpose
Clarify the boundary between kernel logic and boot-artifact layout concerns.

## Kernel Should Not Own
- final image placement policy
- linker-script specifics
- boot artifact packaging rules

## Boot Image Should Own
- linker artifacts
- external entry exposure
- layout assumptions for boot experiments
- target-coupled build details

## Rule
Keep linker ownership out of the kernel core unless a clearly justified internal abstraction is required.
