# =====================================================================
# ADRIAN OS main.rs Reconciliation Pack v1
# ---------------------------------------------------------------------
# Purpose:
#   Fix the compile error by rewriting ONLY axiom/boot-image/src/main.rs
#   so it calls exclusively the function / type names that ACTUALLY EXIST
#   in the current module files on disk. No other file is touched.
#
#   Verified public API (from your own signature dump):
#     entry.rs:
#       wrapper_entry_status(), entry_phase_label()
#     bridge.rs:
#       SyntheticBootContext{ version, arch_label, memory_map_present,
#                             serial_available, experiment_mode },
#       SyntheticBootContext::fbe1_default(), is_well_formed(),
#       bridge_status(), bridge_context_status(&ctx)
#     invoke.rs:
#       CrossingOutcome{ ... }, perform_simulated_crossing(&ctx),
#       invoke_status(), crossing_status(&outcome)
#     handoff.rs:
#       SyntheticHandoff::fbe1_default(), handoff_status(),
#       handoff_summary(&h), handoff_transition_relation()
#     transition.rs:
#       transition_status(), transition_boundary_label(),
#       transition_candidate_phase_label(),
#       transition_marker_proof_relation()
#     candidate.rs:
#       TransitionCandidate::mrt1_current(),
#       ready_for_real_crossing_gate(), is_experiment_only(),
#       candidate_status(), candidate_summary(&c)
#
# Run from: D:\adrian-os
#   cd D:\adrian-os
#   .\adrian-os-mainrs-reconciliation-pack-v1.ps1
# =====================================================================

$ErrorActionPreference = "Stop"

$root = (Get-Location).Path
Write-Host "ADRIAN OS root: $root" -ForegroundColor Cyan

if (-not (Test-Path (Join-Path $root "Cargo.toml"))) {
    Write-Host "ERROR: No Cargo.toml found at $root." -ForegroundColor Red
    Write-Host "Run this from D:\adrian-os (the workspace root)." -ForegroundColor Red
    exit 1
}

$bootImageSrc = Join-Path $root "axiom\boot-image\src"
New-Item -ItemType Directory -Force -Path $bootImageSrc | Out-Null

$mainFile = Join-Path $bootImageSrc "main.rs"

$mainContent = @'
//! ADRIAN OS boot-image wrapper: placeholder binary entry.
//!
//! Compile-clean placeholder. This file calls ONLY functions that exist
//! in the current module files. It surfaces the wrapper-side conceptual
//! flow, the synthetic BootContext, the synthetic handoff, the
//! transition boundary, the MRT-1 candidate, and the first simulated
//! boundary crossing. No real Axiom code runs; no bootable artifact is
//! produced.

mod entry;
mod bridge;
mod invoke;
mod flow;
mod handoff;
mod transition;
mod candidate;

fn main() {
    // --- Wrapper-side entry (WF-1) ---
    println!("{}", entry::wrapper_entry_status());
    println!("{}", entry::entry_phase_label());

    // --- Bridge (WF-2) + synthetic BootContext (Minimal v1) ---
    println!("{}", bridge::bridge_status());
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

    // --- Invoke (WF-3) ---
    println!("{}", invoke::invoke_status());

    // --- FBE-1 / flow framing ---
    println!("{}", flow::wrapper_flow_summary());
    println!("{}", flow::wrapper_semantic_chain_summary());
    println!("{}", flow::mrt1_coordination_summary());

    // --- Synthetic handoff model (FMH-1) ---
    let handoff_model = handoff::SyntheticHandoff::fbe1_default();
    println!("{}", handoff::handoff_status());
    println!("{}", handoff::handoff_summary(&handoff_model));
    println!("{}", handoff::handoff_transition_relation());

    // --- Transition boundary (MRT-1) ---
    println!("{}", transition::transition_status());
    println!("{}", transition::transition_boundary_label());
    println!("{}", transition::transition_candidate_phase_label());
    println!("{}", transition::transition_marker_proof_relation());

    // --- Structured MRT-1 transition candidate ---
    let cand = candidate::TransitionCandidate::mrt1_current();
    println!("{}", candidate::candidate_status());
    println!("{}", candidate::candidate_summary(&cand));
    println!(
        "candidate-gate: ready_for_real_crossing_gate={} experiment_only={}",
        cand.ready_for_real_crossing_gate(),
        cand.is_experiment_only()
    );

    // --- First simulated boundary crossing (same-crate stub target) ---
    let outcome = invoke::perform_simulated_crossing(&boot_context);
    println!("{}", invoke::crossing_status(&outcome));
    println!(
        "crossing-outcome: entry_reached={} marker_count={} halt_reached={} simulated={}",
        outcome.entry_reached,
        outcome.marker_count,
        outcome.halt_reached,
        outcome.simulated
    );
}
'@

Set-Content -Path $mainFile -Value $mainContent -Encoding UTF8
Write-Host "Wrote (reconciled): $mainFile" -ForegroundColor Green

# --- Sanity: no malformed (bracket/paren) filenames anywhere ----------
$weird = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '[\[\]\(\)]' }
$weirdCount = ($weird | Measure-Object).Count
Write-Host ("Weird file count: " + $weirdCount) -ForegroundColor Yellow
if ($weirdCount -gt 0) {
    Write-Host "Malformed files found (review before deleting):" -ForegroundColor Red
    $weird | ForEach-Object { Write-Host ("  " + $_.FullName) -ForegroundColor Red }
}

Write-Host ""
Write-Host "main.rs Reconciliation Pack v1 applied." -ForegroundColor Cyan
Write-Host "Now run, from D:\adrian-os:" -ForegroundColor Cyan
Write-Host "  cargo check" -ForegroundColor White
Write-Host '  git add .' -ForegroundColor White
Write-Host '  git commit -m "main.rs Reconciliation Pack v1: align main.rs calls with actual module APIs"' -ForegroundColor White
Write-Host "  git push" -ForegroundColor White
