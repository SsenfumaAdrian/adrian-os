# First Simulated Boundary Crossing v1

## Purpose
Perform the FIRST experiment-bounded boundary crossing on the wrapper
side using a SAME-CRATE SIMULATED target. This proves the call SHAPE
compiles and locks the wrapper -> entry calling convention BEFORE real
cross-crate linkage to adrian-kernel is introduced.

## What was added (invoke.rs)
- `CrossingOutcome` â€” outcome struct shaped like a real Axiom early path
  report: entry_reached, marker_count, halt_reached, simulated.
- `axiom_entry_stub(&SyntheticBootContext)` â€” LOCAL stand-in for
  `axiom::entry::kernel_entry(&BootContext)`. Validates the context and
  returns an 8-marker modeled outcome. No real I/O, no real halt.
- `perform_simulated_crossing(&SyntheticBootContext)` â€” wrapper-side
  driver that validates then invokes the stub.
- `crossing_status(&CrossingOutcome)` â€” status line for surfacing.

## Modeled Axiom marker sequence (count = 8)
ENTRY, BOOT CONTEXT OK, ARCH INIT, MM INIT, SECURITY INIT, IPC INIT,
SCHED INIT, HALT.

## Safety invariants
- SIMULATED only: the target is a local stub, NOT real Axiom.
- No cross-crate dependency on adrian-kernel yet.
- No QEMU, no bootable artifact.
- candidate.real_axiom_call stays FALSE (stub != real Axiom).
- candidate.real_boot_context stays FALSE (context is synthetic).
- No real serial I/O, no real halt.

## Why simulate first
Locking the call shape and outcome contract in-crate de-risks the next
step (real cross-crate linkage), where the wrapper and Axiom BootContext
shapes must actually unify. If the simulated shape is right, the real
crossing becomes a substitution rather than a redesign.

## Guarantees
- Compile-clean (`cargo check` expected to pass)

## Status
- Phase: MRT-1 post-cohesion, first simulated crossing done
- Crossing gate: open; simulated crossing exercised; real crossing pending
- Next: real cross-crate linkage option â€” add adrian-kernel as a
  dependency and unify BootContext shapes, replacing the stub with the
  real axiom::entry::kernel_entry (still no QEMU at that step)
