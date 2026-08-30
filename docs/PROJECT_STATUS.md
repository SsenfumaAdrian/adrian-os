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

**Last updated:** 2026-08-30, after adding the bare-metal image crate
(roadmap step 2).

**Verification status of this update: mixed, and the split matters.**
Everything this document said before today still rests on GitLab pipeline
[#2803579010](https://gitlab.com/adrian-group9612635/adrian-os/-/pipelines)
on commit `66bab0e`, which passed all six jobs in 1m34s — `rust-build`,
`rust-test`, `boot-test`, `dart-analyze` (×2), `graph-validate`.

`rian/bare-metal` is **partly verified, and the split is now precise
rather than pessimistic.** `image-build` ran on commit `cd8d57b0` and
failed at exit 101, but not before establishing two things that had only
ever been arguments: the kernel compiles for `x86_64-unknown-none` (rustc
1.98.0, clean), and it does so with `feature="default"` alone, so the
workspace exclusion really does keep `std` from unifying onto it. What
failed is `src/main.rs`, at macro expansion: `boot.s` contained a curly
brace inside the very comment documenting that it must not, and
`global_asm!` parses braces as format arguments. Fixed, with a grep guard
added to both remotes because rustc reports that error against the
`include_str!` call site and never names the offending line.

Still not verified, and the failure stopped the build before any of it:
LLVM's integrated assembler has not seen `boot.s`, `rust-lld` has not used
`linker/rian.ld`, and nothing has booted — `image-boot` `needs:
image-build`. The local evidence remains what it was: assembly and linking
by GNU binutils as a *proxy*, `tools/image/`'s 28 checks run against that
proxy and shown falsifiable, and `tools/graph/validate.py`'s four new
checks (16 → 20) guarding the two properties of the crate that no compiler
can see.

This is the first CI log ever read for this project, and it retires the
largest caveat this document used to carry. Until this run, every
"unit tested" claim anywhere in the repo rested on tests written to be
correct rather than on an observed pass. They have now been observed. The
218 `#[test]` functions across the three test-bearing crates (168 kernel,
31 pulse, 19 vault) compile and pass, in both the hosted and the `no_std`
configurations, and the boot sequence ran end to end.

What that does **not** establish is unchanged and still the main point of
this document: nothing here has executed outside a hosted userspace
process. See the verification table above — `boot tested (hosted)` is now
a level this project has actually reached, and `boot tested (bare metal)`
is still one it cannot reach at all.

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
`vault`. `rian/bare-metal` (`adrian-bare-metal`) is a fifth Rust crate
held deliberately *outside* the workspace via `exclude`; `canvas` sits
outside any workspace.

**What exists and is unit tested** — and, as of pipeline #2803579010,
observed to pass rather than merely believed to: a spin lock with a real 8-thread ×
10,000-increment concurrency test; a fixed-capacity physical-region
classifier and bootstrap allocator; x86_64 IDT, PIC remap, PIT divisor
math, and 4-level page-table types; a round-robin ready queue; process
and thread tables over a unified handle registry; IPC channels and
events; capability rights and security labels with `is_authorized` wired
into syscall dispatch; the vault key envelope.

**What exists but is not reachable from a real boot:** all of the above.
Nothing has ever executed outside a hosted test process. What *has* now
been established, by pipeline #2803579010, is that the hosted boot path
runs to completion: `cargo run -p adrian-boot-image` exited 0, which it
only does when init reports `Ready` **and** leaves behind a
ten-stage, in-order, non-overflowed trace. So the init sequence is
`boot tested (hosted)` — nine subsystem init functions genuinely execute,
in the documented order, and say so.

**The single most important fact about this repository:** no code here has
ever run in ring 0 on any machine, real or emulated. Every "x86_64" module
is currently type-checked hardware description, not exercised hardware
bring-up.

That fact is unchanged, but the reason for it has changed. Until
2026-08-30 there was no bare-metal build path at all — no linker script,
no `.cargo/config.toml`, no target configuration, no entry stub. There now
is one: `rian/bare-metal` (see below). What is missing is no longer the
path but the evidence, and the two must not be conflated. Nothing in that
crate has been through `rustc`.

A claim this document used to make and should not have: "`rian/halo/` is a
directory, not a bootloader." `rian/halo/` does not exist. `halo/` is at
the **repository root** and contains a single `README.md` describing an
intended trust chain. The substance of the original point stands — there is
no bootloader — but the path was wrong, and a wrong path in a status
document is how a reader ends up verifying the wrong thing.

**The bare-metal artifact — `rian/bare-metal`, `not yet verified`.** A
fifth crate, deliberately outside the workspace. It contains a multiboot1
entry stub (`src/boot.s`) that clears the bss, checks for long mode,
identity-maps the first 1 GiB with 2 MiB pages, loads a flat 64-bit GDT and
calls into Rust; a linker script placing the image at 1 MiB; a
`.cargo/config.toml` selecting `x86_64-unknown-none` with
`-C code-model=small -C relocation-model=static`; and `src/main.rs`, which
translates the bootloader handoff into a `BootContext` and calls
`entry::kernel_entry_and_halt`. It is excluded from the workspace because
it is `#![no_main]` (a root `cargo build` would fail on a missing `main`)
and because resolver-2 feature unification with `rian/boot-image` would
silently give it a `std` kernel.

Verification level, stated precisely, because this is the claim most likely
to be over-read:

- **`src/boot.s` assembles and `linker/rian.ld` links** — established
  locally with GNU `as --64` and `ld -n -T`, against a stub `rian_main`.
  That is evidence about assembly syntax, operand legality and linker
  script correctness. It is **not** evidence that LLVM's integrated
  assembler accepts the file, and it is not a Rust build.
- **The linked layout is what it should be** — one `LOAD` segment at
  VirtAddr = PhysAddr = 0x100000, `.bss` NOBITS of 0x13010 bytes,
  `MemSiz − FileSiz` = 81,906, `_start` at 0x10000c immediately after the
  12-byte header, `boot_pml4` 4 KiB aligned, `boot_stack_top` 16-byte
  aligned. The 32-bit machine encodings were hand-decoded, because
  `objdump` renders a `.code32` section of an ELF64 under 64-bit rules and
  its output for this file is misleading.
- **The kernel compiles for `x86_64-unknown-none`: verified in CI**, job
  `image-build` on commit `cd8d57b0`, rustc 1.98.0. This is the first time
  the kernel has been compiled for a bare-metal target at all — previously
  the `no_std` configuration was only ever built for the host with
  `--no-default-features`, which is a different thing. `adrian-kernel`
  built clean, no warnings shown, with `-C panic=abort -C opt-level=3`.
- **The workspace exclusion demonstrably works.** The `--verbose` rustc
  line for `adrian-kernel` in that job reads `--cfg 'feature="default"'`
  and nothing else: `std` did **not** unify onto the kernel. That was the
  entire reason for excluding the crate, it was previously an argument from
  how resolver 2 is documented to behave, and it is now an observation.
- **`.cargo/config.toml` is found, and all four of its settings reach
  rustc** — `--target x86_64-unknown-none`, `-C link-arg=-Tlinker/rian.ld`,
  `-C code-model=small`, `-C relocation-model=static` all appear on the
  command line in the job log. The mandatory `cd` works as described.
- **`src/main.rs` does not compile yet** — `image-build` failed at exit 101
  with `invalid asm template string: expected closing brace`, reported
  against `main.rs:27`. Cause: `boot.s` contained a curly brace, in the
  comment paragraph documenting the rule that it must not. Fixed, and
  `image-build` now greps for braces before building, because rustc's
  diagnostic names the `include_str!` call site and never the offending
  line. Consequence for everything below it: LLVM's integrated assembler
  has still not seen `boot.s`, `rust-lld` has still not used
  `linker/rian.ld`, and whether `compiler_builtins` supplies the
  `memcpy`/`memset` the kernel needs remains open — the build never
  reached assembly, let alone linking.
- **Boot: `not yet verified`.** `image-boot` did not run; it `needs:
  image-build`.

**Image verification tooling — `tools/image/`, verified as tooling.**
`verify_shape.sh` asserts 16 properties of the linked ELF; `verify_boot.sh`
asserts 12 properties of the serial log. Both have been run and both were
shown to be *falsifiable*, which is the only thing that makes a green run
mean anything: each of the 28 checks was made to fail by perturbing its
subject — discarding the multiboot section, moving the load address,
splitting LMA from VMA, misaligning `boot_pml4` by one byte, shrinking the
stack, stripping symbols, linking 32-bit, patching `e_type` to DYN,
`ld -q`, and fourteen hand-built serial logs including an empty one, a
reordered boot trace and a duplicated stage. That pass found a real bug: a
`${pml4:-0}` fallback meant the PML4 alignment check silently *passed* on a
stripped image, since 0 is 4 KiB aligned. Fixed to `:-1`.

Both scripts were re-run end to end against a freshly rebuilt proxy after
the fix — shape 16/16, boot 12/12 on both LF and CRLF logs — because the
first pass predated the `:-1` change and a green result from a script that
has since been edited is not evidence about the script that ships. That
re-run also corrected three perturbations of my own that had looked like
passing checks and were nothing of the kind: `verify_boot.sh` greps for the
exact markers the kernel emits, and `panic.rs` emits `RIAN PANIC` with no
colon, so a hand-written log saying `RIAN: PANIC:` left the panic check
green while proving nothing. The lesson is about perturbation, not about the
script: a perturbation has to be built from the string the source actually
contains, or the falsifiability pass silently tests nothing.

**How this project gets compiled.** No local environment both links
executables and runs tests: MSVC can type-check libraries but cannot
link, and the GNU toolchain is blocked by Smart App Control. **GitLab CI
is therefore the only thing that compiles this project.** GitHub Actions
was the intended co-equal second remote and its workflow is committed and
correct, but the account's billing is locked, so GitHub refuses to run
any workflow at all — it currently verifies nothing and must not be cited
as evidence. That makes a single GitLab pipeline the sole support for
every verification claim in this document, which is a real fragility and
not a footnote.

GitLab checks four things: workspace build, kernel-alone build (which is
the only place the `no_std` configuration gets compiled at all — under
resolver 2, `boot-image`'s `features = ["std"]` unifies onto the kernel
for any workspace-wide invocation), the full test suite, and, as of
Sprint 1, a hosted boot test. It additionally runs `dart analyze` and
`tools/graph/validate.py`. Two further jobs are committed but have not yet
run: `image-build` (adds the `x86_64-unknown-none` target, builds
`rian/bare-metal`, runs `verify_shape.sh`, publishes the ELF) and
`image-boot` (installs QEMU, boots the image with `-serial file:`, runs
`verify_boot.sh`). Until they report, the bare-metal crate is
`not yet verified` in this document's exact sense.


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

**P0 — the bare-metal build path exists but is unproven.** Downgraded
today from "no bare-metal build path at all", and downgraded only in its
description. `rian/bare-metal` now supplies every piece that was listed as
missing: `x86_64-unknown-none` (chosen over a custom target JSON because it
is stable, tier 2, and links with `rust-lld`, which matters given Smart App
Control blocks the GNU toolchain), a `.cargo/config.toml`, an entry stub
that establishes a stack before any Rust runs, and a linker script. Still
missing: a bootloader of our own — the image is loaded by `qemu -kernel`
via a multiboot1 header, and `halo/` remains a README.

This stays P0 because the claim "every `arch::x86_64` module is
unverifiable in principle" is retired only when a pipeline says the image
builds and boots, and no pipeline has. Written-but-uncompiled is a
different state from absent, and it is not a better one until the compiler
agrees. Retire this entry when `image-build` and `image-boot` are green,
and not before.

**P0 — no GDT, no exception handlers, no loaded IDT.** `Idt::new()` is
constructed and dropped; `Idt::load()` is never called. Any fault
therefore escalates to a triple fault and resets the machine with no
diagnostic. This is the first thing that must work after a boot exists,
because without it every other bug presents as a silent reboot.

The entry stub loads a flat 3-entry GDT, which is the minimum needed to
leave compatibility mode and is not the GDT this entry is about: no TSS, no
ring-3 descriptors, no IST stacks. And it makes this problem *more*
pressing rather than less — the image now has a path to a real fault on
real silicon, and no way to say anything about it when one arrives.
`verify_boot.sh`'s "the serial log is not empty" check exists precisely
because a triple fault during bring-up is the one failure that produces no
output at all.

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

**Retired: the verification gap.** Previous revisions of this document
ended section C with "no CI log from either remote has ever been read",
called it the single largest source of uncertainty here, and were right
to. Pipeline #2803579010 closed it. The remaining uncertainty is of a
different and more honest kind: not "does this code compile and pass its
own tests" — it does — but "do those tests assert anything about
hardware", and for every `arch::x86_64` module the answer is still no.
That is what P0 above is about, and no amount of green CI will change it
until there is something to boot.

---

## D. Master roadmap

Ordered so that each step is verifiable when it lands, rather than
grouped by subsystem.

1. **Boot observability** — make init report what it did. *(Sprint 1 —
   done, and verified by pipeline #2803579010: boot tested (hosted).)*
2. **Bare-metal artifact** — `x86_64-unknown-none`, entry stub, linker
   script, minimal bootloader; QEMU target in CI. First step that
   produces `boot tested (bare metal)` as an achievable verification level.
   *(Written 2026-08-30 as `rian/bare-metal` plus `tools/image/`; **not yet
   verified** — awaiting the `image-build` and `image-boot` jobs. "Minimal
   bootloader" was descoped to a multiboot1 header loaded by `qemu -kernel`,
   which is a deliberate narrowing: writing a bootloader and booting a
   kernel are two unverified things, and doing both at once means a failure
   cannot say which one broke. `halo/` is where a real one goes.)*
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

**Delivered** (all **verified** by GitLab pipeline #2803579010 on
`66bab0e` — compiled in both feature configurations, tests passed, and
the hosted boot ran end to end):

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
  Sprint 1 change that was verified *before* CI existed, because it is
  Python and the environment runs Python. Sprint 1 broke a check: the assertion
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
*legible* — and pipeline #2803579010 proves it is legible in practice and
not merely in intent. Step 2 of the roadmap is what makes it *real*.

---

## F. Roadmap step 2 — the bare-metal artifact

**Status: written, not yet verified.** Read section A's verification
breakdown before citing anything here. Nothing below has been compiled by
`rustc` or booted by anything.

**Goal.** Produce the ELF a bootloader loads, so that `boot tested (bare
metal)` stops being a level this project cannot reach. Sprint 1 made init
report on itself; that reporting is worth having only if there is a machine
to report from.

**Delivered.**

- **`rian/bare-metal/src/boot.s`.** Multiboot1 header; bss clear with `rep
  stosb` before the stack — which lives inside the bss — is claimed; the
  handoff parked in reserved slots across the mode switch; a CPUID extended-
  leaf and long-mode check with single-character COM1 diagnostics on
  failure; an identity map of the first 1 GiB using 2 MiB pages across three
  fixed 4 KiB tables (no allocator needed, which matters because
  `early_mm_init(&[])` still has nothing); CR4.PAE → EFER.LME → CR0.PG in
  that order; a flat 3-entry GDT; and `retf` into a 64-bit code segment.
- **`linker/rian.ld`.** Header first and `KEEP()`-ed, image at 1 MiB,
  `__bss_start`/`__bss_end` exported, `.got`/`.got.plt` placed rather than
  discarded so that their ever being non-empty is visible instead of
  silent, unwind and note sections discarded.
- **`.cargo/config.toml`.** `x86_64-unknown-none`, the linker script, and
  `-C code-model=small -C relocation-model=static` — the target's defaults
  assume a higher-half relocatable kernel, and a DYN image with nobody to
  apply its relocations is one of the failure modes that builds cleanly.
- **`src/main.rs`.** `rian_main`, the UART brought up before anything is
  said through it, a handoff report, and the call to
  `entry::kernel_entry_and_halt`. `handoff_label` is split out as a pure
  `const fn` so there is a return value something could assert on.
- **Excluded from the workspace,** from both sides. Reasons in section A;
  the cost is that root `cargo build` does not cover it, which is why
  `image-build` exists.
- **`tools/image/verify_shape.sh`** — 16 assertions on the linked ELF,
  between "it built" and "it booted". Every way a bare-metal image can be
  unloadable is silent: a DYN image, a garbage-collected multiboot header, a
  load address the loader will not honour, a PML4 that lost its alignment.
  POSIX `sh` over `readelf` and `od` on purpose — those exist in every CI
  image that can build Rust; `llvm-tools` does not.
- **`tools/image/verify_boot.sh`** — 12 assertions on the serial log,
  including the one worth having: the ten boot stages compared *as an
  ordered sequence*, so a duplicate, an omission and a reorder are all
  caught by one comparison. Reads a log rather than a QEMU exit code
  because the image contains no `isa-debug-exit` write, and it should not:
  a port write whose only purpose is to tell a harness the answer is
  test-only code in the artifact that ships.
- **CI, both remotes.** `image-build` and `image-boot` on GitLab, split so
  a failure names which half broke; one combined `image` job on GitHub,
  where splitting would mean artifact upload/download machinery on a remote
  that cannot run anyway.

**Verified locally, with the method stated.** There is no `rustc` and no
QEMU in the environment this was written in, so the honest boundary is:
`boot.s` assembles under GNU `as --64` and links under `ld -n -T rian.ld`
against a stub `rian_main`, producing exactly the intended layout (one LOAD
at VirtAddr = PhysAddr = 0x100000, `MemSiz − FileSiz` = 81,906, `_start` at
0x10000c, `boot_pml4` 4 KiB aligned, `boot_stack_top` 16-byte aligned), and
`verify_shape.sh` passes 16/16 against it. GNU `as` is a *proxy* for LLVM's
integrated assembler, not the same assembler. The 32-bit encodings were
hand-decoded because `objdump` renders a `.code32` section of an ELF64
under 64-bit rules and its output for this file is actively misleading.

**Both verification scripts were shown to be falsifiable** — all 28 checks,
each turned red by perturbing its own subject, listed in the header comment
of each script. This found one real bug (a `${pml4:-0}` fallback made the
PML4 alignment check pass on a stripped image, since 0 is 4 KiB aligned)
and one useful non-result: `ld -shared` on this image fails with
`R_X86_64_32 against __bss_start ... recompile with -fPIC`, which means
`relocation-model=static` is not a preference the linker tolerates but a
requirement the image cannot be built without.

**The connection analyser gained four checks, 16 → 20**, and they exist
because two properties of this crate are invisible to every other form of
verification the project has. The first is the workspace exclusion: a build
that gets it wrong *succeeds*, handing the image a `std` kernel through
resolver-2 feature unification, so no `cargo` invocation can report it and
`validate.py` asserting `non_workspace_crates == ["adrian-bare-metal"]` is
the only check in the tree that can see it. The second is that this is the
one crate with no tests of its own, so nothing would notice if `rian_main`
quietly stopped calling into the kernel; three edge assertions
(`adrian-bare-metal → adrian-kernel::boot / ::debug / ::entry`, matching the
three modules `main.rs` actually imports and all three load-bearing) close
that. All four were confirmed falsifiable by six perturbations of a throwaway
copy of the tree: promoting the crate to a workspace member turned the
exclusion check red, removing each kernel import in turn turned exactly its
own edge check red, reducing `main.rs` to a stub that calls nothing turned all
three red, and deleting the crate turned all four red.

**Not delivered.** No IDT, TSS or exception handler, so any fault from
`rian_main` onward is a triple fault that resets the machine and truncates
the serial log — that is step 3 and it is the next thing that should exist.
The multiboot memory map is requested by the header and not parsed, so
`entry_count` stays 0 — step 4. No framebuffer. No tests inside the crate,
because `cargo test` would build it for the host and it does not link for
the host; `tools/image/` is where its tests live instead.

