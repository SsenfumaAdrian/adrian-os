# Progress

Current, honest state of every subsystem. "Verified" means actually
built and tested — via unit tests, hand-traced algorithms, official
RFC test vectors, or an end-to-end `cargo run`, depending on what's
appropriate for the piece. Nothing here is claimed as working without
saying how it was checked. Last updated after adding the bare-metal image
crate (`rian/bare-metal` plus `tools/image/`), which is **written and not
yet compiled** — see the new section below and do not read it as delivered.

> **See also [`docs/PROJECT_STATUS.md`](docs/PROJECT_STATUS.md)** for the
> ranked problem list, master roadmap, and per-sprint record. This file
> stays the per-subsystem ledger; that one is the planning view. Where
> they overlap, they must agree — if they disagree, one of them is stale.

## rian/kernel

| Subsystem | Status |
|---|---|
| Boot path | Hosted dev-loop wrapper (`rian/boot-image`) crosses into the real kernel entry point, and now **returns**: it prints the boot trace and exits 0 only if init reported `Ready` *and* left a complete, in-order trace. That makes it a CI step (`cargo run -p adrian-boot-image`) rather than something you had to Ctrl+C. (**boot tested (hosted)** — pipeline #2803579010.) A bare-metal path now exists on disk as `rian/bare-metal`, but **no `rustc` has seen it** — see the section below. |
| Boot observability | Real: `boot_trace::BootStage`/`BootTrace` record each of the 10 init stages in order, with `is_ordered` (catches duplicates and reordering), `is_complete`, and overflow counted rather than panicked on. Init takes its recorder as a parameter, so tests assert against a local trace instead of racing on the global. This is what gave the previously-untested nine-step init sequence coverage at all. (**unit tested** — pipeline #2803579010.) |
| Kernel entry | Real: `BootContext` construction, validation, and the actual `kernel_entry` call, not a simulated stand-in. Exactly one entry point. `kernel_entry` now returns an `InitOutcome` rather than diverging; the bare-metal never-return form is the separately named `kernel_entry_and_halt`, so the halt decision belongs to the caller that knows whether it is firmware or a dev loop. A duplicated context validation with two different failure behaviors was removed. (**unit tested** — pipeline #2803579010.) |
| Memory management | Physical bootstrap allocator (region classification, bump allocation, checked overflow arithmetic), x86_64 4-level page table entry encoding and address splitting, plus `SoftwarePageMapper`: a real table-walking mapper with `map_page`, `unmap_page` and `translate`, handling 1 GiB/2 MiB leaves and rejecting non-canonical addresses. The boot-time memory model is still **not** settled — the mapper takes a `phys_offset` and makes the caller guarantee a direct map of physical memory, so the open question became an explicit parameter rather than going away. Nothing in the boot path can supply that yet, so the mapper has no production callers, only tests. No TLB invalidation and no reclamation of emptied intermediate tables; both are unnecessary until CR3 points at these tables. |
| Interrupts/timers | IDT entry encoding, PIC remap + full mask, PIT frequency/divisor math, all real and tested. No real exception handlers installed and `Idt::load()` is never called — `extern "x86-interrupt"` is nightly-only on this toolchain, and hand-written interrupt entry assembly isn't something to write without a way to verify it (no QEMU here). |
| Serial debug | UART 16550 register programming, and — new — a **bounded** wait on the Transmitter Holding Register Empty bit before each byte. The THRE check was previously computed and discarded (`let _ = transmitter_ready();`), which is why early output arrived mangled. Bounded rather than the textbook unbounded loop because with no UART at 0x3F8 the status register never reports ready and the kernel would wedge on its first debug message. Timeouts are counted, not silently dropped. (**unit tested** — pipeline #2803579010.) |
| Scheduler | Real round-robin ready queue, fixed-capacity ring buffer, thoroughly tested including wraparound. `early_sched_init`'s `debug_assert!(queue.is_empty())` was removed: it asserted a property of a global any earlier caller may legitimately have changed, and holding only in debug builds meant the boot path behaved differently by optimization level. |
| Process/thread | Real creation, destruction, and state tracking, wired to the scheduler — creating a thread actually enqueues it, and if the ready queue is full the spawn is now **unwound** (thread removed, handle unregistered) instead of leaving a `Runnable` thread in no queue, which would have meant boot reporting success while nothing ever ran. Two `.expect()` calls were removed from the boot path; both misdiagnosed the only failure that can actually occur (a *full* table, a runtime condition, not a zero-capacity one). (**unit tested** — pipeline #2803579010.) |
| Syscalls | `ProcessCreate`, `ThreadCreate`, `ChannelCreate`, `EventCreate`, `HandleClose` are all real, dispatching to real kernel functions, and all now go through a capability check first (**unit tested** — pipeline #2803579010). |
| IPC | Real `Channel` (send/receive, backpressure, closed-channel handling) and `Event` (signal/clear) objects, each with a real table and syscall-reachable create/destroy. |
| Security | Real `CapabilityRights` (bitflag composition, the `can_derive` narrowing invariant) and `SecurityLabel` (trust ordering), combined into `is_authorized` — and now enforced: every syscall carries a `SyscallPolicy` (minimum label + required rights) and `dispatch_syscall_as` authorizes the caller's `SyscallContext` against it before any side effect, denying with `PermissionDenied`. What is missing is provenance, not enforcement: nothing populates a `SyscallContext` from hardware state (no trap handler, no current-thread concept), so callers pass it explicitly, and kernel objects carry no label of their own yet — the label check compares against the syscall's minimum, not the target object's trust level. (**unit tested** — pipeline #2803579010.) |
| Handle registry | Real `object::HandleRegistry` mapping any `KernelObjectId` to its kind, letting `HandleClose` dispatch generically across object types. |
| Panics in kernel paths | None left in the init path. `halt_forever()` now parks the core with `hlt` on bare-metal x86_64 instead of spinning at 100% forever (hosted and non-x86_64 keep the spin form, since `hlt` faults outside ring 0), and the dead `panic_handler_placeholder()` — a second, fake panic path next to the real one — was deleted. (**unit tested** — pipeline #2803579010.) |

**Sprint 1 is verified.** GitLab pipeline
[#2803579010](https://gitlab.com/adrian-group9612635/adrian-os/-/pipelines)
on commit `66bab0e` passed all six jobs in 1m34s: `rust-build`,
`rust-test`, `boot-test`, `dart-analyze` (×2), `graph-validate`.

This was the first CI log ever read for this project, and it is worth
being precise about what it did and did not settle. It settled that all
218 `#[test]` functions (168 kernel, 31 pulse, 19 vault) compile and
pass; that the kernel builds in **both** feature configurations, the
`no_std` one included, which only the kernel-alone job exercises; and
that the boot sequence runs end to end, since `cargo run -p
adrian-boot-image` exits 0 only when init reports `Ready` *and* leaves a
complete, in-order, non-overflowed ten-stage trace behind. That is the
first time this project has reached the `boot tested (hosted)`
verification level at all.

It settled nothing about hardware. No code here has executed outside a
hosted userspace process, so every `arch::x86_64` module remains
type-checked hardware description. Green CI cannot change that; only a
bare-metal target can.


A block sat here recording that capability enforcement in `syscall.rs`
had been through `rustc` locally on MSVC but that its tests were still
unrun, and it ended with the instruction "drop the two markers above
once that job is green". Pipeline #2803579010 is that job. Removed
rather than left in place, because a note explaining that something is
unverified becomes actively misleading the moment it is verified. What
it was waiting to confirm — that `dispatch_syscall_as` authorizes
before any side effect, and that the four-argument
`is_authorized(holder_label, holder_rights, target_label,
requested_rights)` order is correct, which is easy to get wrong and
impossible to catch by reading — is now covered by the 21 passing
syscall tests and the 15 security ones.

**Genuinely blocked, not just unstarted:** real exception handlers,
context switching, and a page-table mapper with production callers. All
three need either real/virtualized hardware this sandbox doesn't have, or a
settled design question (the boot-time memory model) that depends on
firmware integration that doesn't exist yet.

"True bare-metal compilation" was the fourth item on that list until
2026-08-30. It is off the list — not because it is done, but because it
turned out not to be blocked. The blocker was assumed to be the local
toolchain, and the actual answer was that the artifact does not need to
build locally: `x86_64-unknown-none` links with `rust-lld`, needs no
external linker, and CI has both a Rust image and QEMU. The lesson is worth
keeping: "blocked by the sandbox" was, in this case, an untested assumption
about the sandbox.

## rian/bare-metal — the image a bootloader loads

**Written 2026-08-30. Not yet compiled, not yet booted.** Read that before
reading anything below it. This section exists so the ledger records the
crate; it records no verification claim about the Rust half, because there
is none to make. Full detail in
[`rian/bare-metal/README.md`](rian/bare-metal/README.md) and section F of
`docs/PROJECT_STATUS.md`.

| Piece | Status |
|---|---|
| `src/boot.s` — multiboot1 header, bss clear, CPUID long-mode check, 1 GiB identity map with 2 MiB pages, CR4.PAE → EFER.LME → CR0.PG, flat 3-entry GDT, `retf` to 64-bit | **Assembles and links** under GNU `as --64` / `ld -n -T`, exit 0, no diagnostics, correct layout. GNU `as` is a *proxy* for LLVM's integrated assembler, which is what `global_asm!` actually uses — so this is evidence about syntax and operand legality, not proof the real build accepts it. 32-bit encodings hand-decoded, because `objdump` reads a `.code32` section of an ELF64 under 64-bit rules and its output here is misleading. |
| `linker/rian.ld` — header first and `KEEP()`-ed, image at 1 MiB, `__bss_start`/`__bss_end` exported | **Links**, and the result is what it should be: one LOAD segment at VirtAddr = PhysAddr = 0x100000, `.bss` NOBITS 0x13010 bytes, `MemSiz − FileSiz` = 81,906, `boot_pml4` 4 KiB aligned, `boot_stack_top` 16-byte aligned. |
| `.cargo/config.toml` — `x86_64-unknown-none`, the linker script, `-C code-model=small -C relocation-model=static` | **Not yet verified.** The two `-C` flags override target defaults that assume a higher-half relocatable kernel. Not optional: `ld -shared` on this image fails with `R_X86_64_32 against __bss_start ... recompile with -fPIC`, so a PIC link is not something the image tolerates. |
| `src/main.rs` — `rian_main`, handoff report, call to `kernel_entry_and_halt` | **Not yet verified.** No `rustc` has seen it. Whether `compiler_builtins` supplies the `memcpy`/`memset` the kernel needs, and whether `rust-lld` links the whole thing, are open until CI runs. |
| `tools/image/verify_shape.sh` — 16 assertions on the linked ELF | **Runs, passes 16/16, and is falsifiable.** Each check was turned red by perturbing its subject (multiboot section discarded, load address moved, LMA split from VMA, `boot_pml4` misaligned by one byte, stack shrunk, symbols stripped, 32-bit link, `e_type` patched to DYN, `ld -q`). The pass found a real bug: a `${pml4:-0}` fallback made the alignment check *pass* on a stripped image, since 0 is 4 KiB aligned. Re-run 16/16 against a proxy rebuilt from scratch *after* that fix, since a green run from a script that has since been edited says nothing about the script that ships. |
| `tools/image/verify_boot.sh` — 12 assertions on the serial log | **Runs, passes 12/12, and is falsifiable** against hand-built logs including an empty one (the signature of a triple fault during bring-up), a reordered boot trace and a duplicated stage; 12/12 on both LF and CRLF, since QEMU emits CRLF and the script normalizes once. The ordering check compares all ten stages as one sequence, so omissions, duplicates and reorderings are caught by a single comparison. Each of the five failure-marker checks was re-perturbed using the exact string its source emits — `panic.rs` writes `RIAN PANIC` with no colon, and a plausible-looking `RIAN: PANIC:` in a hand-written log leaves that check green while testing nothing. |
| `tools/graph/validate.py` — 4 assertions about this crate specifically | **Runs, 20/20, and is falsifiable.** One check asserts the crate is *outside* the workspace, three that it still reaches `adrian-kernel::boot`, `::debug` and `::entry`. Turned red on a throwaway copy of the tree by promoting the crate to a member, by removing each kernel import in turn, by stubbing `main.rs` so it calls nothing, and by deleting the crate. This is the only verification here that runs today and stays honest tomorrow — the other two need an image. |
| CI — `image-build`, `image-boot` (GitLab); combined `image` job (GitHub) | **Committed, never run.** GitLab is split in two so a failure names which half broke; GitHub is one job because splitting there means artifact upload/download machinery on a remote that cannot run at all. |

**What the image deliberately does not have.** No IDT, TSS or exception
handler, so any fault from `rian_main` onward is a triple fault that resets
the machine and truncates the serial log — the next thing that should
exist. The multiboot memory map is requested by the header and not parsed,
so `entry_count` stays 0. No framebuffer. No `isa-debug-exit` write: a port
write whose only purpose is to tell a test harness the answer would be
test-only code inside the artifact that ships, so the harness reads the
serial log instead. No tests inside the crate, because `cargo test` would
build it for the host and it does not link for the host.


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

`ServiceSupervisor` now bundles the state machine, restart policy and
health policy behind one type, with `handle_failure` and
`evaluate_health`. Two things to be clear about: it is a façade, holding
no failure history of its own — the caller owns that and passes it in,
which is what keeps the decision stateless and testable — and the
"backoff" in the restart policy is a failure *count* inside a sliding
window, not a delay. Nothing here damps a crash loop in time yet. It has
no callers outside its own test.

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
Dependency Pins below); **key storage** (needs real persistent storage,
none exists); and the actual **attestation/boot-chain usage** these
primitives would eventually back.

**Key generation exists but currently has no provider to call it with —
by design.** `SymmetricKey::generate` takes an `EntropyProvider`, and the
only implementation of that trait in the crate is `MockEntropySource`, a
counter that fills a key with `seed, seed+1, seed+2, …`. Because
`counter` is a `u8` its entire keyspace is **256 keys**.

It used to be `pub` and ungated, which made the generation API reachable
from production code with test-grade entropy and nothing stopping it.
It is now behind `#[cfg(any(test, feature = "test-utils"))]`, with
`test-utils` off by default and enabled nowhere in this workspace. The
consequence is deliberate: in a normal build `EntropyProvider` has **no
implementors at all**, so `generate` type-checks but cannot be called.
An API that is uncallable is a better state than one that can only be
called wrongly, and it stays that way until a real entropy source
exists. How keys actually get generated, rotated and stored is still
open — see below.

*Verification status: the gating is a `cfg` change plus a new Cargo
feature, not yet through `rustc` here — CI settles whether `cargo build
-p adrian-vault` (feature off) and `cargo test` (feature implied by
`cfg(test)`) both still compile.*

`KeyEnvelope` itself is sound where it counts: real ChaCha20-Poly1305,
the domain tag genuinely passed as AAD (so a wrong tag fails), tag
comparison constant-time inside the crate, and plaintext released only
after verification. Two gaps: the `version` field is checked but **not
authenticated** (it is outside the AAD, so once a v2 exists an attacker
with write access to stored envelopes could force a v1 downgrade), and
`seal` takes a caller-supplied nonce with no uniqueness mechanism or
warning — nonce reuse under one key breaks ChaCha20-Poly1305 completely.
`SymmetricKey` also does not zeroize on drop, though `zeroize` is
already a declared dependency.

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

**Not compiler-verified locally, but CI now checks it.** No Dart SDK is
available in this sandbox — checked both apt and GitHub releases, neither
has it; Dart's actual distribution goes through Google's own
infrastructure, outside this sandbox's allowed network list. The code was
written carefully (bracket and string-literal balance checked
mechanically, only long-stable Dart idioms used) but has never been run
through a Dart toolchain here.

That gap is now closed on the CI side rather than locally: both remotes
run `dart pub get` + `dart analyze --fatal-infos` over `sdk/dart` and
`canvas`, so a compile break can no longer land green. `dart test` is
deliberately not run yet — `canvas` declares a `test` dev-dependency but
ships no `test/` directory, so the step would fail for a reason unrelated
to code health. **The first run of these jobs has not been read yet**, so
"analyze-clean" is not yet a claim this document makes; it is a check that
now exists.

## Tooling

**graphify** (github.com/Graphify-Labs/graphify) was used for codebase
navigation during the `pulse` split and the early structural passes —
run code-only (fully local, no LLM calls, zero token cost). It is **not
runnable in either environment this project is currently worked on
from**: its runtime needs `networkx`, `rapidfuzz` and ~28 tree-sitter
grammar packages, and the agent sandbox has no PyPI access. A read-only
clone sits at `graphify/` (gitignored, carries its own `.git`) so the
source can still be consulted. The PyPI package is **`graphifyy`**, not
`graphify`.

It earned trust before being relied on: it correctly identified the real
structural hubs (`BootContext`, `Channel`, `dispatch_syscall()`),
confirmed zero import cycles, and gave accurate line-numbered call sites
on direct query. It was also caught giving a false positive once
(`ChannelState`/`MessageHeader`/`EventObject` flagged as "isolated" when
each has 5-7 real usages — a blind spot around struct-field and
enum-variant relationships, not actual dead code). Its cohesion-score
flag was accurate, though, and drove the `pulse/src/lib.rs` module
split. Output lived in `graphify-out/` (gitignored).

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
20/20 passing. The count rose from 13 to 16 in Sprint 1 (two new checks
assert that boot observability is actually wired, and one hub check was
split into a ranking half and a rank-independent reach half after
`boot_trace` legitimately displaced `BootContext` from the top 8), then to
20 with `rian/bare-metal` (one check that the crate is *outside* the
workspace, three that it still reaches `adrian-kernel::boot`, `::debug` and
`::entry`). Those four are there because no Rust build can see either
property: a workspace-membership mistake compiles and links, and the crate
has no tests of its own to notice `rian_main` becoming a stub.
It is deliberately narrower than graphify: Rust only, no
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
  direction. A first pass now exists (`LiquidGlassTheme` design tokens
  with `crystal`/`obsidian` presets, and a `GlassNode` element tree), but
  it renders nothing: `canvas` is not in any workspace and is not
  referenced by the Dart SDK. It is at least *analysed* by CI now, so it
  can no longer break silently — but nothing consumes it. The direction
  itself is still the owner's call.
- **Real encryption key management** — `SymmetricKey::generate` exists,
  and its only entropy source, `MockEntropySource`, is now gated behind
  the `test-utils` feature, so the mock is fenced off from production and
  `generate` has no callable provider in a normal build. What remains a
  design question is the part gating cannot answer: how keys actually get
  generated, rotated, and stored, and where the real entropy comes from
  on a machine with no OS services yet.
- **Whether `dart test` should be a required check** — analysis runs now,
  but tests do not. `sdk/dart` has a `test/` directory; `canvas` has none,
  so turning the step on today would fail there for a reason unrelated to
  code health. Either write `canvas` tests first or scope the step to
  `sdk/dart` — a call worth making deliberately rather than by default.

## Orphaned files (Cleaned Up)

All six `.rs` scaffold files previously tracked outside the Cargo workspace (`rian/security/mod.rs`, `rian/ipc/mod.rs`, `rian/arch/arm64/mod.rs`, `rian/arch/x86_64/mod.rs`, `rian/mm/mod.rs`, `rian/sched/mod.rs`) have been cleaned up and removed. `tools/graph/validate.py` confirms 20/20 checks passing with zero orphaned files remaining outside the workspace. Note that "outside the workspace" now means two different things and the check distinguishes them: a loose `.rs` file belonging to no crate is the orphan this section is about and must stay at zero, while `rian/bare-metal` is a whole crate excluded on purpose and is asserted to be there.

## How this project gets compiled

Worth recording plainly, because "did this compile?" is the question
this document exists to answer honestly, and the answer depends on
where you are:

| Environment | Rust build | Why |
|---|---|---|
| Agent sandbox (Linux) | No | No rustc, no cargo, and no network to install them. |
| Windows host, MSVC toolchain | **Libraries only** | `rustc.exe` runs fine. `cargo build -p adrian-kernel` succeeds in both feature configurations, because an `rlib` needs no linker at all. Anything that produces an executable — `cargo test`, or building `adrian-boot-image` — fails with `error: linker 'link.exe' not found`. The MSVC linker ships with Visual Studio Build Tools, not with rustup. |
| Windows host, GNU toolchain | No | Installs, then refuses to execute: `An Application Control policy has blocked this file. (os error 4551)` — Smart App Control blocking a low-reputation unsigned binary. |
| **GitHub Actions** | **No — account billing locked** | `.github/workflows/rust.yml` is present and correct, but as of 2026-08-30 GitHub refuses to run it: *"GitHub Actions workflows can't be executed on this repository. Your account's billing is currently locked."* The workflow is kept, not deleted, because the block is an account state rather than a defect — it starts working again the moment billing is resolved. Until then it verifies nothing, so do not cite it as evidence. |
| **GitLab CI** | **Yes — the only working compiler** | `.gitlab-ci.yml` on the `gitlab` remote. Mirrors the GitHub workflow step for step and additionally runs `python3 tools/graph/validate.py` and `dart analyze`. The two-remote setup was built for redundancy and has now paid for itself: one remote is unavailable and the other still answers "does it build". Keep them in sync anyway — each file says so in a comment — so that GitHub resumes as a real second opinion rather than as bit-rot. |

So **GitLab CI is currently the only thing in the world that compiles
this project.** That is a single point of failure for every verification
claim in this document, and worth fixing when convenient — either by
restoring GitHub billing or by getting a linker onto the Windows host.

So **CI is the compiler for this project**, not a safety net on top of
local builds. Read the GitLab Pipelines log before believing any Rust
claim here — and note that the GitHub Actions log is not an alternative
source, because it does not exist.

One subtlety that makes CI trustworthy for the kernel specifically. The
workflow runs a bare `cargo test --verbose` with no `--features` flag,
which looks like it would skip `adrian-kernel`'s tests, since the crate
is `#![cfg_attr(not(feature = "std"), no_std)]` and its test harness
needs `std`. It does not skip them: `rian/boot-image/Cargo.toml`
declares `adrian-kernel = { path = "../kernel", features = ["std"] }`,
and `resolver = "2"` still unifies features across normal dependencies
within one invocation, so a workspace-root `cargo test` builds the
kernel *with* `std`. That is also why the narrower command needs the
flag spelled out — `-p adrian-kernel` drops boot-image from the graph,
and the feature goes with it:

```
cargo test -p adrian-kernel --features std
```

The same subtlety is why the bare-metal image is a separate crate outside
the workspace and a separate CI job. Feature unification is per-invocation,
so a `rian/bare-metal` inside the workspace would receive the kernel *with*
`std` from `boot-image`, silently, and still link — the one configuration a
bare-metal image must never have. The cost is that no root-level command
covers it: `cargo build`, `cargo test` and `cargo run` at the repository
root do not touch that crate at all. `image-build` is what covers it, and it
must `cd rian/bare-metal` first, because the target, the linker script and
the code and relocation models all live in that crate's
`.cargo/config.toml`, which cargo finds by walking up from the working
directory. Run from the root, all four settings are silently absent and the
build fails on a missing `main` for the host target.

Two things follow that are easy to get wrong. A `.cargo/config.toml` at the
*repository* root setting `build.target` would look like a convenience and
would retarget the entire workspace, including the hosted test suite. And
`verify_shape.sh`/`verify_boot.sh` are POSIX `sh` over `readelf`, `od` and
`awk` rather than `llvm-tools`, because the first three exist in every CI
image that can build Rust and the last does not.

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
