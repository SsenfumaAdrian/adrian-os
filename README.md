# ADRIAN OS

A from-scratch operating system. Rust kernel core (codename **Rian**,
pulled from "Ad-**RIAN**"), Dart application layer, Python for tooling
and automation. Hybrid modular architecture, capability-based
zero-trust security throughout, macOS-inspired "Liquid Glass" visual
design language on top of an entirely native stack — nothing borrowed
from other OSes.

Status, honestly: this does not boot on real hardware yet. What
exists is a large, genuinely tested body of kernel logic and
supporting crates, verified everywhere it's possible to verify
without real or virtualized hardware. See [PROGRESS.md](PROGRESS.md)
for the detailed, current state of every subsystem — what's real and
tested, what's stubbed, and exactly why each open item is still open.

## Platform Direction

- Desktop and laptop class devices first; mobile and embedded later
- x86_64 with UEFI boot first; ARM64 planned second
- Curated hardware support before broad compatibility
- Security and architectural integrity before ecosystem expansion

## Initial Strategy

1. Establish kernel architecture and boot chain
2. Build minimal but real drivers for curated hardware
3. Implement the service and security model
4. Build the SDK and initial application layer
5. Expand hardware and application support only after the core is
   stable and secure

## Repository Areas

| Path | Purpose |
|---|---|
| `halo/` | Boot and trust chain |
| `rian/` | Kernel and core primitives (formerly codenamed Axiom — see [PROGRESS.md](PROGRESS.md) for the rename) |
| `pulse/` | Service manager |
| `vault/` | Security, crypto, identity |
| `sentinel/` | Policy and sandboxing |
| `current/` | Networking |
| `prism/` | Graphics and compositor |
| `canvas/` | UI framework |
| `flow/` | Animation and motion |
| `orbit/` | Packaging and updates |
| `nexus/` | Cloud and fleet services |
| `sdk/` | Developer platform (Dart application SDK) |
| `tools/` | Build, CI, test, automation |
| `apps/` | First-party applications |

## Current Status

A summary; [PROGRESS.md](PROGRESS.md) has the full detail, including
what's genuinely verified versus what's still open and why.

- **`rian/kernel`** — boot path, memory management (physical
  allocator + x86_64 page table encoding), interrupt/timer scaffold
  (IDT/PIC/PIT, real handlers still pending), a round-robin scheduler,
  process/thread creation, syscalls, IPC (channels and events), and
  capability-based security policy. All real, tested, and connected —
  `cargo run` executes the full sequence end to end. Does not run on
  real or virtualized hardware yet: no bare-metal target build, no
  installed exception handlers, no context switching.
- **`pulse`** — all five responsibilities from its own original scope
  (boot graph, dependency resolution, restart policy, lifecycle
  transitions, health supervision) have real, tested decision logic.
  Doesn't yet manage a real running service, since that needs the
  kernel-side execution gaps above closed first.
- **`vault`** — real, RFC-verified cryptographic primitives:
  ChaCha20-Poly1305 authenticated encryption (RFC 8439), HKDF-SHA256
  key derivation (RFC 5869), Ed25519 signing (RFC 8032), all via
  audited RustCrypto crates rather than hand-rolled implementations.
  Key generation, storage, and the actual attestation/boot-chain usage
  are still open — they need a real entropy source and real
  persistent storage, neither of which exists yet.
- **`sdk/dart`** — a typed application API (process/thread/channel/
  event lifecycle) over an in-memory host simulation. Not yet
  toolchain-verified; see PROGRESS.md.
- **`halo`, `sentinel`, `current`, `prism`, `canvas`, `flow`, `orbit`,
  `nexus`, `apps`** — not yet started beyond their own README.

## Building & Testing

Requires a Rust toolchain (`cargo`, `rustc`) and, for `vault`, no
extra setup — its dependencies are pinned to versions compatible with
a standard stable toolchain (see [vault/Cargo.toml](vault/Cargo.toml)
if you're on an older one and hit a resolution error; PROGRESS.md
documents the specific dependency pins and why).

```sh
# Build everything
cargo build

# Run the whole test suite for one crate
cargo test -p adrian-kernel --features std
cargo test -p adrian-pulse
cargo test -p adrian-vault

# Run the kernel's hosted dev-loop wrapper end to end
cargo run -p adrian-boot-image
```

`adrian-kernel` builds in two configurations: the default (`no_std`,
what a real bare-metal build uses) and `--features std` (a hosted
simulation used for testing and for `adrian-boot-image`'s dev loop —
see `rian/boot-image/src/main.rs` for what that boundary actually
means).

## Documentation

- [PROGRESS.md](PROGRESS.md) — detailed, current-state tracking of
  every subsystem
- [CONTRIBUTING.md](CONTRIBUTING.md) — how to contribute, coding
  standards, verification expectations
- [SECURITY.md](SECURITY.md) — how to report a vulnerability
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- `docs/` — architecture, design decisions, interfaces, and the
  historical record of the kernel's early bring-up phase (written
  when it was still codenamed Axiom)

## License

Not yet chosen. Until a license is added, all rights are reserved by
default — this is an open item, not an oversight; see PROGRESS.md.
