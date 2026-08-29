# Progress

Current, honest state of every subsystem. "Verified" means actually
built and tested — via unit tests, hand-traced algorithms, official
RFC test vectors, or an end-to-end `cargo run`, depending on what's
appropriate for the piece. Nothing here is claimed as working without
saying how it was checked. Last updated alongside the Axiom → Rian
rename.

## rian/kernel

| Subsystem | Status |
|---|---|
| Boot path | Hosted dev-loop wrapper (`rian/boot-image`) crosses into the real kernel entry point. No bare-metal target build yet — this sandbox has no path to one (see Blockers). |
| Kernel entry | Real: `BootContext` construction, validation, and the actual `kernel_entry` call, not a simulated stand-in. |
| Memory management | Physical bootstrap allocator (region classification, bump allocation, checked overflow arithmetic) plus x86_64 4-level page table entry encoding and address splitting. No table-walking mapper yet — needs the boot-time memory model settled, which needs real firmware/Halo integration. |
| Interrupts/timers | IDT entry encoding, PIC remap + full mask, PIT frequency/divisor math, all real and tested. No real exception handlers installed and `Idt::load()` is never called — `extern "x86-interrupt"` is nightly-only on this toolchain, and hand-written interrupt entry assembly isn't something to write without a way to verify it (no QEMU here). |
| Scheduler | Real round-robin ready queue, fixed-capacity ring buffer, thoroughly tested including wraparound. |
| Process/thread | Real creation, destruction, and state tracking, wired to the scheduler — creating a thread actually enqueues it. |
| Syscalls | `ProcessCreate`, `ThreadCreate`, `ChannelCreate`, `EventCreate`, `HandleClose` are all real, dispatching to real kernel functions, and all now go through a capability check first (**unverified** — see below). |
| IPC | Real `Channel` (send/receive, backpressure, closed-channel handling) and `Event` (signal/clear) objects, each with a real table and syscall-reachable create/destroy. |
| Security | Real `CapabilityRights` (bitflag composition, the `can_derive` narrowing invariant) and `SecurityLabel` (trust ordering), combined into `is_authorized` — and now enforced: every syscall carries a `SyscallPolicy` (minimum label + required rights) and `dispatch_syscall_as` authorizes the caller's `SyscallContext` against it before any side effect, denying with `PermissionDenied`. What is missing is provenance, not enforcement: nothing populates a `SyscallContext` from hardware state (no trap handler, no current-thread concept), so callers pass it explicitly, and kernel objects carry no label of their own yet — the label check compares against the syscall's minimum, not the target object's trust level. (**unverified** — see below.) |
| Handle registry | Real `object::HandleRegistry` mapping any `KernelObjectId` to its kind, letting `HandleClose` dispatch generically across object types. |

**Unverified, pending a compile:** the capability enforcement in
`syscall.rs` (`SyscallPolicy`, `SyscallContext`, `dispatch_syscall_as`,
and its 8 new tests) has not been through `rustc`. It was written in an
environment with no Rust toolchain, so the only checks it has passed are
structural: brace/paren balance, and `tools/graph/validate.py` asserting
that `adrian-kernel::syscall` really does now depend on
`adrian-kernel::security` and that `is_authorized` has production
callers outside its own module. Neither is a substitute for
`cargo test -p adrian-kernel --features std`. Drop the two markers above
once that passes.

**Genuinely blocked, not just unstarted:** real exception handlers,
context switching, a page-table mapper, and true bare-metal
compilation. All four need either real/virtualized hardware this
sandbox doesn't have, or a settled design question (the boot-time
memory model) that depends on firmware integration that doesn't exist
yet.

## pulse

All five responsibilities from its original README now have real,
tested logic, split one module per concern
(`manifest.rs`/`restart.rs`/`lifecycle.rs`/`health.rs`):

- **Dependency resolution** — Kahn's algorithm, hand-traced against a
  linear chain, a diamond dependency, and cycles before trusting the
  tests.
- **Restart policy** — sliding-window crash-loop backoff, boundary
  cases (exactly at the limit, exactly at the window edge) tested
  explicitly, not just the comfortable middle.
- **Lifecycle transitions** — a minimal state machine, checked
  exhaustively across all 25 (from, to) pairs against an
  independently-built expected set.
- **Health supervision** — a three-tier (healthy/degraded/unhealthy)
  heartbeat-recency policy, distinct in shape from restart policy
  (recency of one signal vs. frequency across a history).

None of this manages a real running service yet — that needs the
kernel-side execution gaps closed first (context switching, at
minimum).

## vault

Three cryptographic primitives, each via an audited RustCrypto crate
and checked against the official standard, not just self-consistency:

- **Encryption** — ChaCha20-Poly1305 AEAD. Matches RFC 8439 §2.8.2's
  official test vector byte-for-byte (all 130 bytes of expected
  output), plus tamper detection on both ciphertext and associated
  data, and wrong-key/wrong-nonce rejection.
- **Key derivation** — HKDF-SHA256. Matches two RFC 5869 vectors
  (§A.1 basic case, §A.3 zero-length salt/info edge case).
- **Signing** — Ed25519. Matches two RFC 8032 §7.1 vectors (empty
  message, one-byte message), each checked three ways: the
  seed-derived public key, the produced signature bytes, and
  successful verification.

Not attempted: **AES-GCM** specifically (the audited crate's only
stable release needs a Cargo edition this toolchain can't support —
ChaCha20-Poly1305 was the working alternative, not a downgrade — see
Dependency Pins below); **key generation** (needs a real entropy
source, none exists); **key storage** (needs real persistent storage,
none exists); and the actual **attestation/boot-chain usage** these
primitives would eventually back.

### Dependency pins (why, precisely)

This sandbox's Rust toolchain is fixed at rustc 1.75.0 with no path to
upgrade. Several current RustCrypto releases require Cargo's
`edition2024` feature, unavailable at that version, with no newer
stable line to fall back to in some cases. Resolved by tracing each
dependency chain to an older, compatible version rather than
abandoning the audited-crate approach:

- `zeroize = "=1.8.1"` (chacha20poly1305 pulled in 1.9.0 by default,
  which needs edition2024; 1.8.1 satisfies every crate's `^1.8`
  requirement and predates that requirement)
- `hkdf = "=0.12.4"` (0.13 needs `crypto-common ^0.2`, which needs
  edition2024; 0.12.4 uses the older, compatible `^0.1` line already
  in use elsewhere in this dependency graph)
- `ed25519-dalek = "=2.1.1"` with `default-features = false, features
  = ["fast", "zeroize"]` (2.2.0 declares rustc 1.81 as its own
  minimum, unrelated to editions; 2.1.1 is the newest version that
  still builds here, and dropping `std`/`rand_core` from the default
  feature set avoided a second edition2024 wall through `ed25519`'s
  own `pkcs8`/`spki` chain)

If you have a newer toolchain available (e.g. via Claude Code running
locally with `rustup`), these pins can likely be relaxed — worth
revisiting there rather than assumed fixed forever.

## sdk/dart

A typed `AdrianProcess`/`AdrianThread`/`AdrianChannel`/`AdrianEvent`
API over an in-memory host simulation
(`HostSimulationBackend`), mirroring the same "simulate now, real
bridge later" approach `rian/boot-image` used from its first commit.
Scoped to match what the kernel actually supports — create/destroy
only, since that's all that's syscall-reachable on the Rust side too.

**Not compiler-verified.** No Dart SDK is available in this sandbox —
checked both apt and GitHub releases, neither has it; Dart's actual
distribution goes through Google's own infrastructure, outside this
sandbox's allowed network list. Written carefully (checked bracket/
string-literal balance mechanically, used only long-stable Dart
idioms) but not run through `dart analyze` or `dart test`. Do that
before trusting it further, ideally via Claude Code or another
environment with a real Dart install.

## Tooling

**graphify** (github.com/Graphify-Labs/graphify) is installed and in
use for codebase navigation — run code-only (fully local, no LLM
calls, zero token cost). Verified useful before trusting it: correctly
identified the real structural hubs (`BootContext`, `Channel`,
`dispatch_syscall()`), confirmed zero import cycles, and gave accurate
line-numbered call sites on direct query. Also caught it giving a
false positive once (`ChannelState`/`MessageHeader`/`EventObject`
flagged as "isolated" when each has 5-7 real usages — a real blind
spot around struct-field/enum-usage relationships, not actual dead
code) — worth knowing before trusting its every flag without checking.
Its cohesion-score flag was accurate, though, and drove the
`pulse/src/lib.rs` module split. Output lives in `graphify-out/`
(gitignored, regenerate with `graphify update .`).

**That blind spot is fixed upstream as of graphify 0.9.52.** Its Rust
extractor now walks `field_declaration` type nodes, tuple-struct
positional fields, and enum-variant payloads, and `_rust_collect_type_refs`
recurses through `reference_type`/`array_type`/`tuple_type`/`slice_type`
and into `generic_type` arguments — which is exactly the shape
`slots: [Option<(KernelObjectId, EventObject)>; CAPACITY]` needs. Its own
source comments describe the old behaviour as a bug ("every enum-variant
type reference was silently dropped"). Established by reading
`graphify/extractors/rust.py` in a clone of 0.9.52, **not** by re-running
it against this tree — worth re-confirming on the next real run.

**`tools/graph/`** is a local, stdlib-only connection analyser written
because graphify cannot be installed in every environment this repo gets
worked on from (its runtime needs `networkx`, `rapidfuzz` and ~28
tree-sitter grammar packages). `python3 tools/graph/render.py` writes
`graph-out/graph.{json,html}` — a self-contained offline dashboard, no
CDN, opens from `file://`. `python3 tools/graph/validate.py` checks the
analyser against facts established by hand in this document before it
existed, including a regression guard for the false positive above;
13/13 passing. It is deliberately narrower than graphify: Rust only, no
community detection, no cross-language support. Where they overlap they
agree.

## The Axiom → Rian rename

The kernel's codename changed from Axiom to Rian (pulled from
"Ad**RIAN**"). Renamed everywhere it's live, current-state code or
documentation: the `rian/` directory itself, every debug marker string
(confirmed at runtime, not just in source), and every README inside
that tree. Deliberately **not** renamed:
`docs/interfaces/`, `docs/architecture/`, `docs/roadmap/`, and
`docs/testing/` — the historical process documents from the project's
early bring-up phase, written when the kernel genuinely was called
Axiom at that point. Rewriting those would be revisionist about what
the project was actually called when those design decisions were
made and recorded, the same reasoning as not rewriting old commit
messages.

## Open decisions

Things that need a real choice from the project owner, not a default
this document should assume:

- **License** — none chosen yet; see README.
- **Full security policy model** — `is_authorized` is one reasonable,
  conservative shape (rights narrowing + trust ordering), explicitly
  not claimed as definitive.
- **`canvas`'s actual "Liquid Glass" visual language** — aesthetic
  direction, not yet started.
- **Real encryption key management** — once a real entropy source and
  persistent storage exist, how keys actually get generated, rotated,
  and stored is a design question in its own right.

## Orphaned files, found and left alone

Two near-duplicate scaffold files exist outside the Cargo workspace
entirely, unreferenced by anything: `rian/security/mod.rs` (predates
the rename; was `axiom/security/mod.rs`) and `rian/ipc/mod.rs`.
Confirmed harmless (not compiled, not imported) before leaving them —
likely leftovers from early top-level scaffolding, before
`rian/kernel/src` became where real code lives. Worth a look during a
cleanup pass; not touched here since there's no way to tell from the
code alone whether they're meant for something later.

## Sandbox environment notes

This sandbox's filesystem has reset twice during this project's
development so far -- unpredictably, mid-session, wiping installed
tooling (Rust toolchain, SSH keys, git config) but never anything
already pushed to GitHub. Recovery is quick (reinstall via apt/pip,
regenerate a signing key, get a fresh push token) precisely because
of the commit-and-push-every-verified-increment discipline this
project has followed throughout -- nothing has ever been lost, only
re-set-up. Worth knowing if a session appears to "forget" recent
uncommitted work: check `git log` on a fresh clone before assuming
anything is actually gone.
