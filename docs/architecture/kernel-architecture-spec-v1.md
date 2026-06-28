# ADRIAN OS Kernel Architecture Specification v1

Status: Foundational Draft

## Kernel Codename
Axiom

## Mission
Provide the secure execution core for scheduling, memory management, interrupts, IPC, system calls, and kernel object enforcement.

## Design Direction
- Hybrid modular kernel
- Rust-first implementation
- Minimal unsafe boundaries
- Strong process isolation
- Capability-aware IPC
- SMP support
- Future ARM64 portability

## Core Subsystems
- architecture abstraction
- memory management
- scheduler
- syscall layer
- IPC
- object model
- security hooks
- driver boundary primitives

## Milestones
- K1 boot to kernel main
- K2 memory manager initialization
- K3 interrupts and timers
- K4 scheduler
- K5 first userspace process
- K6 IPC baseline
- K7 capability enforcement baseline
