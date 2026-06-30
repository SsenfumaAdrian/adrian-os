# =====================================================================
# ADRIAN OS First Simulated Boundary Crossing Pack v1
# ---------------------------------------------------------------------
# Purpose:
#   Perform the FIRST experiment-bounded boundary crossing on the wrapper
#   side using a SAME-CRATE SIMULATED target. The wrapper hands the
#   synthetic BootContext to a local stub that stands in for
#   axiom::entry::kernel_entry(&BootContext), proving the call SHAPE
#   compiles and locking the calling convention.
#
# Guarantees:
#   - Compile-clean
#   - SIMULATED only: target is a local stub, NOT the real Axiom kernel
#   - No cross-crate dependency on adrian-kernel yet
#   - No QEMU, no bootable artifact
#   - candidate.real_axiom_call stays FALSE (stub != real Axiom)
#   - candidate.real_boot_context stays FALSE (context is synthetic)
#   - Plain filenames only (no Markdown-link paths)
#   - Explicit directory creation before any Set-Content
#
# Run from: D:\adrian-os
#   cd D:\adrian-os
#   .\adrian-os-first-simulated-boundary-crossing-pack-v1.ps1
# =====================================================================

$ErrorActionPreference = "Stop"

$root = (Get-Location).Path
Write-Host "ADRIAN OS root: $root" -ForegroundColor Cyan

if (-not (Test-Path (Join-Path $root "Cargo.toml"))) {
    Write-Host "ERROR: No Cargo.toml found at $root." -ForegroundColor Red
    Write-Host "Run this from D:\adrian-os (the workspace root)." -ForegroundColor Red
    exit 1
}

$bootImageSrc   = Join-Path $root "axiom\boot-image\src"
$docsInterfaces = Join-Path $root "docs\interfaces"

New-Item -ItemType Directory -Force -Path $bootImageSrc   | Out-Null
New-Item -ItemType Directory -Force -Path $docsInterfaces | Out-Null

$invokeFile = Join-Path $bootImageSrc "invoke.rs"
$mainFile   = Join-Path $bootImageSrc "main.rs"
$docFile    = Join-Path $docsInterfaces "first-simulated-boundary-crossing-v1.md"

# ======================================================================
# invoke.rs
# ----------------------------------------------------------------------
# Adds:
#   - axiom_entry_stub(&SyntheticBootContext) -> CrossingOutcome
#     a local stand-in for axiom::entry::kernel_entry. It returns a
#     marker-sequence-shaped outcome WITHOUT performing real I/O or a
#     real halt.
#   - perform_simulated_crossing(&SyntheticBootContext) the wrapper-side
#     crossing driver that validates then calls the stub.
# Preserves the existing invoke status/relation strings.
# ======================================================================
$invokeContent = @'
//! ADRIAN OS boot-image wrapper: invocation + simulated crossing.
//!
//! This module performs the FIRST experiment-bounded boundary crossing
//! using a SAME-CRATE SIMULATED target. The target `axiom_entry_stub`
//! stands in for the real `axiom::entry::kernel_entry(&BootContext)`.
//!
//! IMPORTANT:
//!   - This is SIMULATED. No real Axiom code runs.
//!   - No real serial I/O and no real halt occur.
//!   - The goal is to prove the CALL SHAPE compiles and to lock the
//!     wrapper -> entry calling convention before real cross-crate
//!     linkage is introduced.

use crate::bridge::SyntheticBootContext;

/// Result of a simulated boundary crossing.
///
/// Mirrors the SHAPE of what a real Axiom early path would conceptually
/// report: whether the entry boundary was reached, how many markers were
/// (conceptually) emitted, and whether a halt was (conceptually) reached.
#[derive(Clone, Copy)]
pub struct CrossingOutcome {
    /// True if the (stub) entry boundary accepted the context.
    pub entry_reached: bool,
    /// Number of Axiom markers the real path WOULD emit, modeled here.
    pub marker_count: u32,
    /// True if the (stub) path conceptually reached HALT.
    pub halt_reached: bool,
    /// Whether this outcome came from the SIMULATED stub (always true).
    pub simulated: bool,
}

impl CrossingOutcome {
    /// A rejected crossing (e.g. malformed context).
    pub const fn rejected() -> Self {
        CrossingOutcome {
            entry_reached: false,
            marker_count: 0,
            halt_reached: false,
            simulated: true,
        }
    }
}

/// Local stand-in for `axiom::entry::kernel_entry(&BootContext)`.
///
/// This is NOT the real Axiom kernel. It validates the synthetic context
/// shape and returns a marker-sequence-shaped outcome. It performs no
/// real I/O and no real halt. The expected Axiom marker sequence is
/// modeled by `marker_count` (ENTRY, BOOT CONTEXT OK, ARCH INIT,
/// MM INIT, SECURITY INIT, IPC INIT, SCHED INIT, HALT = 8 markers).
fn axiom_entry_stub(context: &SyntheticBootContext) -> CrossingOutcome {
    if !context.is_well_formed() {
        return CrossingOutcome::rejected();
    }
    CrossingOutcome {
        entry_reached: true,
        marker_count: 8,
        halt_reached: true,
        simulated: true,
    }
}

/// Wrapper-side driver for the first simulated boundary crossing.
///
/// Validates the synthetic context, then invokes the local stub. Returns
/// the outcome for the caller to surface. No real Axiom call occurs.
pub fn perform_simulated_crossing(context: &SyntheticBootContext) -> CrossingOutcome {
    if !context.is_well_formed() {
        return CrossingOutcome::rejected();
    }
    axiom_entry_stub(context)
}

/// High-level invoke status (preserved).
pub fn invoke_status() -> &'static str {
    "FBE-1 WF-3: wrapper invocation placeholder"
}

/// Invocation phase (preserved).
pub fn invoke_phase() -> &'static str {
    "invocation-phase: experiment-ready"
}

/// Handoff-to-invocation relation (preserved).
pub fn handoff_to_invocation() -> &'static str {
    "handoff-to-invocation: future wrapper-side handoff consumer"
}

/// Invocation-to-summary relation (preserved FMH-1 alignment).
pub fn invocation_to_summary() -> &'static str {
    "invocation-to-summary: invocation is the future consumer of the active FMH-1 synthetic handoff summary"
}

/// Status line describing the simulated crossing outcome.
pub fn crossing_status(outcome: &CrossingOutcome) -> &'static str {
    if outcome.entry_reached && outcome.halt_reached && outcome.simulated {
        "crossing: SIMULATED entry reached | markers-modeled=8 | halt-modeled=true | real-axiom=false"
    } else if outcome.simulated {
        "crossing: SIMULATED rejected (synthetic context not well-formed)"
    } else {
        "crossing: UNEXPECTED non-simulated outcome (invariant violation)"
    }
}
'@

Set-Content -Path $invokeFile -Value $invokeContent -Encoding UTF8
Write-Host "Wrote: $invokeFile" -ForegroundColor Green

# ======================================================================
# main.rs
# ----------------------------------------------------------------------
# Drives the first simulated crossing and surfaces the outcome.
# ======================================================================
$mainContent = @'
//! ADRIAN OS boot-image wrapper: placeholder binary entry.
//!
//! Compile-clean placeholder surfacing the wrapper-side conceptual flow,
//! the FMH-1 summary cohesion, the structured MRT-1 candidate, the
//! synthetic BootContext, and now the FIRST simulated boundary crossing.
//! No real Axiom code runs; no bootable artifact is produced.

mod entry;
mod bridge;
mod invoke;
mod flow;
mod handoff;
mod transition;
mod candidate;

fn main() {
    // Wrapper-side conceptual flow (WF-1 .. WF-3).
    println!("{}", entry::entry_status());
    println!("{}", bridge::bridge_status());
    println!("{}", invoke::invoke_status());

    // FBE-1 / MRT-1 framing.
    println!("FBE-1 wrapper flow: entry -> bridge -> invoke");
    println!("FBE-1 semantic chain: entry -> bridge -> synthetic handoff -> invoke -> future Axiom entry");

    // Synthetic BootContext (Minimal v1) constructed on the wrapper side.
    let boot_context = bridge::SyntheticBootContext::fbe1_default();
    println!("{}", bridge::bridge_context_status(&boot_context));
    println!(
        "boot-context: version={} arch={} mem_map={} serial={} experiment={}",
        boot_context.version,
        boot_context.arch_label,
        boot_context.memory_map_present,
        boot_context.serial_available,
        boot_context.experiment_mode
    );

    // Synthetic handoff model (FMH-1).
    let handoff_model = handoff::SyntheticHandoff::fbe1_default();
    println!("{}", handoff::handoff_status());
    println!("{}", handoff::handoff_summary(&handoff_model));
    println!("{}", handoff::handoff_transition_relation());

    // Transition boundary (MRT-1) + FMH-1 summary alignment.
    println!("{}", transition::transition_status());
    println!("{}", transition::transition_relation());
    println!("{}", transition::transition_phase());
    println!("{}", transition::transition_marker_proof_intent());
    println!("{}", transition::transition_to_summary());
    println!("{}", transition::transition_summary_cohesion());

    // Structured MRT-1 transition candidate (post-cohesion).
    let cand = candidate::TransitionCandidate::mrt1_current();
    println!("{}", candidate::candidate_status());
    println!("{}", candidate::candidate_summary(&cand));
    println!(
        "candidate-gate: ready_for_real_crossing_gate={} experiment_only={}",
        cand.ready_for_real_crossing_gate(),
        cand.is_experiment_only()
    );

    // FIRST simulated boundary crossing (same-crate stub target).
    let outcome = invoke::perform_simulated_crossing(&boot_context);
    println!("{}", invoke::crossing_status(&outcome));
    println!(
        "crossing-outcome: entry_reached={} marker_count={} halt_reached={} simulated={}",
        outcome.entry_reached,
        outcome.marker_count,
        outcome.halt_reached,
        outcome.simulated
    );

    // Invariant restatement: still experiment-only, no real Axiom call.
    println!(
        "invariants: real_axiom_call={} real_boot_context={} (both MUST be false in simulated crossing)",
        cand.real_axiom_call,
        cand.real_boot_context
    );
}
'@

Set-Content -Path $mainFile -Value $mainContent -Encoding UTF8
Write-Host "Wrote: $mainFile" -ForegroundColor Green

# ======================================================================
# docs/interfaces/first-simulated-boundary-crossing-v1.md
# ======================================================================
$docContent = @'
# First Simulated Boundary Crossing v1

## Purpose
Perform the FIRST experiment-bounded boundary crossing on the wrapper
side using a SAME-CRATE SIMULATED target. This proves the call SHAPE
compiles and locks the wrapper -> entry calling convention BEFORE real
cross-crate linkage to adrian-kernel is introduced.

## What was added (invoke.rs)
- `CrossingOutcome` — outcome struct shaped like a real Axiom early path
  report: entry_reached, marker_count, halt_reached, simulated.
- `axiom_entry_stub(&SyntheticBootContext)` — LOCAL stand-in for
  `axiom::entry::kernel_entry(&BootContext)`. Validates the context and
  returns an 8-marker modeled outcome. No real I/O, no real halt.
- `perform_simulated_crossing(&SyntheticBootContext)` — wrapper-side
  driver that validates then invokes the stub.
- `crossing_status(&CrossingOutcome)` — status line for surfacing.

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
- Next: real cross-crate linkage option — add adrian-kernel as a
  dependency and unify BootContext shapes, replacing the stub with the
  real axiom::entry::kernel_entry (still no QEMU at that step)
'@

Set-Content -Path $docFile -Value $docContent -Encoding UTF8
Write-Host "Wrote: $docFile" -ForegroundColor Green

# --- Sanity: no malformed (bracket/paren) filenames -------------------
$weird = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '[\[\]\(\)]' }
Write-Host ("Weird file count: " + ($weird | Measure-Object).Count) -ForegroundColor Yellow

# --- Next steps for the user ------------------------------------------
Write-Host ""
Write-Host "First Simulated Boundary Crossing Pack v1 applied." -ForegroundColor Cyan
Write-Host "Now run, from D:\adrian-os:" -ForegroundColor Cyan
Write-Host "  cargo check" -ForegroundColor White
Write-Host '  git add .' -ForegroundColor White
Write-Host '  git commit -m "First Simulated Boundary Crossing Pack v1: same-crate stub crossing, call shape locked"' -ForegroundColor White
Write-Host "  git push" -ForegroundColor White
