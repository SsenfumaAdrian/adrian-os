# =====================================================================
# ADRIAN OS Wrapper Output Completeness Pack v1 (drift-safe)
# ---------------------------------------------------------------------
# Purpose:
#   Clear dead-code warnings by calling ONLY functions whose names and
#   signatures are 100% confirmed from the current module signature dump.
#   Rewrites ONLY axiom/boot-image/src/main.rs (zero drift risk).
#
#   Deliberately NOT touched here (handled in a follow-up once variant
#   names are confirmed):
#     - flow::stage_label(...) / flow::WrapperStage  (variant names unknown)
#   Those two warnings remain after this pack on purpose. Everything else
#   becomes warning-free.
#
#   Confirmed names exercised:
#     entry::wrapper_entry_status(), entry::entry_phase_label()
#     bridge::bridge_status(), bridge_phase(), bridge_to_handoff(),
#       bridge_to_summary(), bridge_context_status(&ctx),
#       SyntheticBootContext::fbe1_default()
#     invoke::invoke_status(), invoke_phase(), handoff_to_invocation(),
#       invocation_to_summary(), perform_simulated_crossing(&ctx),
#       crossing_status(&outcome)
#     flow::wrapper_flow_summary(), wrapper_semantic_chain_summary(),
#       mrt1_coordination_summary()
#     handoff::handoff_status(), handoff_summary(&h),
#       handoff_transition_relation(), SyntheticHandoff::fbe1_default()
#     transition::transition_status(), transition_boundary_label(),
#       transition_candidate_phase_label(), transition_marker_proof_relation()
#     candidate::TransitionCandidate::mrt1_current(), candidate_status(),
#       candidate_summary(&c), fields version/label/phase_label,
#       ready_for_real_crossing_gate(), is_experiment_only()
#
# Run from: D:\adrian-os
#   cd D:\adrian-os
#   .\adrian-os-wrapper-output-completeness-pack-v1.ps1
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
//! Compile-clean placeholder that exercises the confirmed wrapper-side
//! functions. It surfaces the wrapper-side conceptual flow, the synthetic
//! BootContext, the synthetic handoff, the transition boundary, the
//! MRT-1 candidate, and the first simulated boundary crossing. No real
//! Axiom code runs; no bootable artifact is produced.
//!
//! Note: flow::WrapperStage / flow::stage_label are intentionally not
//! exercised yet (variant names to be wired in a follow-up step).

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
    println!("{}", bridge::bridge_phase());
    println!("{}", bridge::bridge_to_handoff());
    println!("{}", bridge::bridge_to_summary());
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
    println!("{}", invoke::invoke_phase());
    println!("{}", invoke::handoff_to_invocation());
    println!("{}", invoke::invocation_to_summary());

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

    // --- Structured MRT-1 transition candidate (all fields read) ---
    let cand = candidate::TransitionCandidate::mrt1_current();
    println!("{}", candidate::candidate_status());
    println!("{}", candidate::candidate_summary(&cand));
    println!(
        "candidate-fields: version={} label={} phase_label={}",
        cand.version,
        cand.label,
        cand.phase_label
    );
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
Write-Host "Wrote (completeness, drift-safe): $mainFile" -ForegroundColor Green

# --- Sanity: no malformed (bracket/paren) filenames anywhere ----------
$weird = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '[\[\]\(\)]' }
$weirdCount = ($weird | Measure-Object).Count
Write-Host ("Weird file count: " + $weirdCount) -ForegroundColor Yellow

Write-Host ""
Write-Host "Wrapper Output Completeness Pack v1 applied (drift-safe)." -ForegroundColor Cyan
Write-Host "Expected after cargo check: only the flow::WrapperStage /" -ForegroundColor Cyan
Write-Host "flow::stage_label warnings should remain (2 warnings)." -ForegroundColor Cyan
Write-Host "Now run, from D:\adrian-os:" -ForegroundColor Cyan
Write-Host "  cargo check" -ForegroundColor White
Write-Host '  git add .' -ForegroundColor White
Write-Host '  git commit -m "Wrapper Output Completeness Pack v1: exercise confirmed wrapper functions"' -ForegroundColor White
Write-Host "  git push" -ForegroundColor White
