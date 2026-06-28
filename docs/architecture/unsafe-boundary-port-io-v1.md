# ADRIAN OS Unsafe Boundary: x86_64 Port I/O v1

## Purpose
Document the first real unsafe hardware boundary introduced into Axiom.

## Why Unsafe Is Required
x86_64 port I/O uses machine instructions that cannot be expressed as ordinary safe Rust operations.

## Boundary Rules
- unsafe confined to architecture-specific module
- no direct random port access from generic kernel code
- serial backend consumes the abstraction, not raw asm
- every future expansion must document invariants and usage scope

## Current Scope
- Port::read_u8()
- Port::write_u8()

## Future Review Items
- timing/delay concerns
- wider port widths if needed
- ordering semantics
- audit of all consumers
