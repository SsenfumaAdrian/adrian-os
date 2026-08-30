# ADRIAN OS — Project Status

Maintained per the master engineering directive (§26). This is the
single place that answers "where is this project actually at". It states
verification level for every claim, using these terms only:

| Term | Means |
|---|---|
| **not yet verified** | Written, never passed through a compiler |
| **compiled** | `cargo build` / `cargo check` accepted it |
| **unit tested** | A `#[test]` asserts its behavior and passed |
| **integration tested** | Multiple subsystems exercised together and passed |
| **boot tested (hosted)** | `cargo run -p adrian-boot-image` ran init end to end and exited 0 |
| **boot tested (bare metal)** | Ran on QEMU or hardware |

There is deliberately no "works" or "done". Compiling is not working.

**Last updated:** 2026-08-30, after the Sprint 1 boot-path refactor.
**Verification status of this update:** the Rust changed in Sprint 1 has
**not yet been verified** — no rustc, cargo, or Dart SDK exists in the
environment these edits were made from. CI on GitHub and GitLab is the
compiler for this project. Nothing below claims otherwise.

---

## A. Current state

**Size.** 6,997 lines of Rust across 35 files in the four workspace
crates (kernel 5,566 / boot-image 101 / pulse 735 / vault 595), plus Dart
(`sdk/dart`, `canvas`) and Python tooling (`tools/graph`). A count of
"7,082 across 30" appeared in an earlier revision of this file; it was
wrong in both figures — it summed in the 85 lines of Rust test fixtures
belonging to `graphify/`, a read-only third-party clone kept in the tree
that `tools/graph/analyze.py` deliberately skips, and it undercounted
files. Corrected here rather than quietly, since a size claim is exactly
the kind of thing that gets copied forward without being re-derived.

**Cargo workspace** (`resolver = "2"`), four members: `rian/kernel`
(`adrian-kernel`), `rian/boot-image` (`adrian-boot-image`), `pulse`,
`vault`. `canvas` sits outside any workspace.

**What exists and is unit tested:** a spin lock with a real 8-thread ×
10,000-increment concurrency test; a fixed-capacity physical-region
classifier and bootstrap allocator; x86_64 IDT, PIC remap, PIT divisor
math, and 4-level page-table types; a round-robin ready queue; process
and thread tables over a unified handle registry; IPC channels and
events; capability rights and security labels with `is_authorized` wired
into syscall dispatch; the vault key envelope.

**What exists but is not reachable from a real boot:** all of the above.
Nothing has ever executed outside a hosted test process.

**The single most important fact about this repository:** there is no
bare-metal build path. No linker script, no `.cargo/config.toml`, no
custom target JSON, no assembly entry stub, no bootloader. `rian/halo/`
is a directory, not a bootloader. So no code here has ever run in ring 0
on any machine, real or emulated. Every "x86_64" module is currently
type-checked hardware description, not exercised hardware bring-up.

**How this project gets compiled.** No local environment both links
executables and runs tests: MSVC can type-check libraries but cannot
link, and the GNU toolchain is blocked by Smart App Control. GitHub
Actions and GitLab CI are the source of truth, and they now check four
things: workspace build, kernel-alone build (which is the only place the
`no_std` configuration gets compiled at all — under resolver 2,
`boot-image`'s `features = ["std"]` unifies onto the kernel for any
workspace-wide invocation), the full test suite, and, as of Sprint 1, a
hosted boot test.

---

## B. Architecture

Four layers, each with a deliberate reason to be separate.

**`rian/kernel` — the kernel proper, `no_std` by default.** A `std`
feature swaps in host-safe equivalents for exactly two privileged
things: raw x86 port I/O (`in`/`out` are ring-0 instructions and fault
from userspace) and the `#[panic_handler]` lang item (a hosted binary
already has std's). Everything else compiles identically in both
configurations, which is what makes hosted tests evidence about the real
kernel rather than about a simulator.

**`rian/boot-image` — the hosted dev loop.** Builds a real `BootContext`
and calls the real `entry::kernel_entry`. It exists so the
firmware→kernel call path can be built and exercised before firmware
exists. As of Sprint 1 it terminates with a meaningful exit code instead
of running forever, which is what makes it usable as a CI step.

**`vault` / `pulse` — userspace-side crates.** Key envelopes and
supervision. `vault`'s `MockEntropySource` is behind a `test-utils`
feature, so with the feature off the `EntropyProvider` trait has zero
implementors and `SymmetricKey::generate` is uncallable by construction —
the intended state, since a 256-value keyspace must never be reachable
in production.

**Recurring internal patterns, each adopted for a specific reason:**

*Fixed-capacity arrays behind `SpinLock`, not collections.* There is no
`#[global_allocator]`, so no `alloc::collections` type is available.
Every table (`ProcessTable`, `ThreadTable`, `RunQueue`, `HandleRegistry`,
`BootTrace`) is a `[Option<T>; N]` with a compile-time assertion that its
bring-up bound never exceeds the eventual `config::KERNEL_MAX_*` target.

*Const-generic capacity so globals are testable.* Every table is generic
over its capacity, so a test can drive a 1- or 2-slot instance and reach
the full-table path that a 64-slot global makes untestable.

*Dependency injection where a global would make tests flake.* `cargo
test` runs tests as parallel threads in one process, so several tests
driving one mutable global interleave. `init::run_init` therefore takes
its progress recorder as an `FnMut` parameter, and `serial::spin_until`
takes its readiness check as one. Production passes the real global-
writing function; tests pass a closure over local state. Same code path,
no shared state.

*Consistent lock ordering.* Object tables are acquired before the handle
registry, and the thread table before the ready queue. There is no
inversion anywhere in the crate, which is why nesting these locks is
currently safe.

*Report, do not panic.* Boot-path failures return values. There are no
`unwrap`/`expect`/`panic!` calls left in the kernel's init path.

---

## C. Critical problems

Ranked by whether they block the next real capability.

**P0 — no bare-metal build path at all.** Nothing produces a bootable
artifact. Missing: a custom target (or `x86_64-unknown-none`, a stable
tier-2 target that uses `rust-lld` and so needs no external linker —
which matters given Smart App Control blocks the GNU toolchain), a
`.cargo/config.toml`, an entry stub that sets up a stack before Rust
code runs, a linker script, and a bootloader. Until this exists, every
`arch::x86_64` module is unverifiable in principle, not just unverified.

**P0 — no GDT, no exception handlers, no loaded IDT.** `Idt::new()` is
constructed and dropped; `Idt::load()` is never called. Any fault
therefore escalates to a triple fault and resets the machine with no
diagnostic. This is the first thing that must work after a boot exists,
because without it every other bug presents as a silent reboot.

**P1 — the bootstrap allocator is seeded with nothing.** `early_mm_init(&[])`
passes an empty region slice, because there is no firmware memory map to
pass. Honest, but it means the allocator can hand out nothing, so no
subsystem can grow past its fixed array.

**P1 — kernel objects carry no security label.** `object.rs` tracks id
and kind only. So `is_authorized` compares the caller's label against
the *syscall's* declared minimum, never against the target object's own
trust level — the label half of capability enforcement is structurally
incomplete. (Tracked as an open task.)

**P1 — no syscall provenance.** `SyscallContext` is passed in explicitly
by whoever calls dispatch, because there is no privilege-transition trap
handler and no current-thread concept to read a caller identity from.
Enforcement is real; identity is not yet.

**P2 — no context switching.** The ready queue orders tasks; nothing
saves or restores register state, so "scheduling" currently means
choosing an id.

**P2 — UTF-8 BOMs on many source files**, and a set of `*.ps1` "pack"
scripts in the repository root that appear to be superseded scaffolding.

**Verification gap, not a code defect:** no CI log from either remote has
ever been read. Every claim of "unit tested" above rests on tests that
were written to be correct, not on an observed green run. This is the
single largest source of uncertainty in this document.

---

## D. Master roadmap

Ordered so that each step is verifiable when it lands, rather than
grouped by subsystem.

1. **Boot observability** — make init report what it did. *(Sprint 1, done pending compile.)*
2. **Bare-metal artifact** — `x86_64-unknown-none`, entry stub, linker
   script, minimal bootloader; QEMU target in CI. First step that
   produces `boot tested (bare metal)` as an achievable verification level.
3. **Fault survivability** — GDT, a real `'static` IDT, exception
   handlers that print a diagnostic. Turns crashes into messages.
4. **Real memory map** — firmware/Halo hands over regions; the allocator
   is seeded with something.
5. **Virtual memory** — load CR3, kernel address space, guard pages.
6. **Interrupts live** — install handlers, unmask, `sti`, PIT tick.
7. **Context switching** — save/restore state; the ready queue starts
   meaning something.
8. **Syscall provenance** — trap handler, current-thread concept, labels
   on objects; capability enforcement becomes complete.
9. **Userspace** — ring 3, ELF loading, a first process.
10. **Filesystem, storage, drivers, networking** — in that order, each
    behind the capability model.
11. **Shell and utilities.**
12. **Reproducible builds and release artifacts.**

---

## E. Sprint 1 — boot path: observable, asserted, panic-free

**Goal.** Before adding capability, make the capability that exists
report on itself. Nine subsystem init functions had zero test coverage
because the only path that reached them ended in an infinite loop, and
nothing can be asserted about a function that never returns.

**Delivered** (all **not yet verified** — awaiting CI):

- **`boot_trace.rs` (new).** `BootStage` (10 ordered variants) and
  `BootTrace`, a fixed-capacity ordered record with `reached`,
  `is_ordered` (each stage strictly later than the last, so a duplicate
  or a reorder is caught), `is_complete`, and `overflowed`. Overflow is
  counted, never panicked on and never overwriting: losing a diagnostic
  always beats taking down the boot that was reporting it. 7 tests,
  including one pinning `BootStage::ALL` against the enum discriminants
  so the ordering assertions cannot silently become vacuous.
- **`init.rs`.** Init no longer decides to halt — that is the entry
  point's job, and removing it is what made the sequence testable.
  Returns `InitOutcome` (`Ready`, `InvalidBootContext`,
  `ProcessInitFailed`, `ThreadInitFailed`) with a `label()` for
  `no_std` logging. `run_init` takes its recorder as a parameter.
  `enter_idle_placeholder()` deleted. 4 tests: full sequence completes
  in order; a bad magic stops at exactly one stage; a wrong version is
  also rejected; init survives running twice.
- **`entry.rs`.** Returns `InitOutcome` instead of diverging; the
  bare-metal "never come back" form is the separate, explicitly named
  `kernel_entry_and_halt`. Also removed a duplicated context validation
  that had two different failure behaviors. 2 tests — the invalid-handoff
  path is now an assertion instead of a hang.
- **`process.rs`, `thread.rs`.** Two `.expect()` calls removed from the
  boot path. Both claimed a zero-capacity table was the only possible
  failure; both functions actually fail when the table is *full*, a
  runtime condition, so the panic message misdiagnosed the only case
  that can occur. Additionally, `make_runnable`'s return value was being
  discarded, which could leave a bootstrap thread marked `Runnable` while
  sitting in no queue — boot reporting success and then nothing ever
  running. Now `spawn_runnable` unwinds the spawn on a full queue,
  including unregistering the handle. 5 new tests.
- **`sched.rs`.** `early_sched_init`'s `debug_assert!(queue.is_empty())`
  removed: it asserted a property of a global that an earlier caller is
  entitled to have changed, and it only held in debug builds, making the
  boot path behave differently by optimization level. Also rewrote a
  test that asserted the *global* queue was empty — once init tests
  enqueue into it, that assertion makes parallel test order load-bearing.
- **`debug/serial.rs`.** `let _ = transmitter_ready();` — a THRE check
  computed and thrown away, which is why early output arrived mangled —
  replaced with a **bounded** wait. Bounded because the textbook
  unbounded form hangs the kernel on its first debug message if no UART
  is present at 0x3F8, before it has any other way to say why. Timeouts
  are counted, not silently dropped. 6 tests.
- **`panic.rs`.** `halt_forever()` now parks the core with `hlt` on
  bare-metal x86_64 instead of spinning a core at 100% forever; hosted
  and non-x86_64 keep the spin form, since `hlt` from userspace faults.
  Dead `panic_handler_placeholder()` deleted.
- **`port_io.rs`.** Both `unsafe` blocks given real SAFETY comments
  stating the CPL-0 invariant and why each `asm!` option is accurate,
  per directive §14.
- **CI, both remotes.** New `cargo run -p adrian-boot-image` step: the
  first end-to-end check that boot *runs* rather than merely compiles.
  It exits non-zero unless init reported `Ready` **and** left a complete,
  in-order trace behind — two independent checks, because the outcome is
  init's opinion of itself while the trace is evidence of the steps it
  reached, and a disagreement between them is exactly what this catches.
- **`tools/graph/validate.py`.** 13 checks → 16, and this is the one
  Sprint 1 change that *is* verified, because it is Python and the
  environment runs Python. Sprint 1 broke a check: the assertion
  "BootContext ranks among the most depended-on symbols" used a fixed
  top-8 window, and `boot_trace`'s `BootStage` (hub score 70) and
  `record` (35) both outrank `BootContext` (30), pushing it to rank 9 of
  200. The recorded fact was genuinely stale rather than the tool wrong,
  so rather than deleting the check or silently widening it: the window
  moved to 12 with a comment naming the two symbols that displaced it
  and why that is the intended outcome of making boot observable, and
  the claim's actual substance — that the firmware→kernel handoff type
  is depended on from *several* modules, which no ranking can be gamed
  into — was split out as a separate rank-independent check. Two further
  checks assert boot observability is wired rather than self-contained:
  an `init → boot_trace` module edge, and `InitOutcome` being referenced
  from `adrian-boot-image`. Without those, `boot_trace` dominating the
  hub ranking above would be measuring nothing. All four new assertions
  were confirmed falsifiable, not vacuous, before being accepted.

**Not delivered, and not claimed:** none of this is bare-metal verified,
because there is still nothing to boot. Sprint 1 makes the boot sequence
*legible*; step 2 of the roadmap is what makes it *real*.
