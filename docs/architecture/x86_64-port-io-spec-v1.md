# ADRIAN OS x86_64 Port I/O Specification v1

## Purpose
Define the low-level x86_64 port I/O abstraction boundary for early hardware communication.

## Why This Exists
Port I/O is inherently architecture-specific and security-sensitive.
It must be isolated from generic kernel code and handled through a controlled abstraction.

## Initial Scope
- u8 port reads/writes first
- serial backend as first consumer
- placeholder-safe implementation before real hardware access
- centralization of unsafe hardware interactions later

## Future Scope
- real in/out instruction implementation
- unsafe invariant documentation
- serial backend integration
- other early hardware consumers if required
