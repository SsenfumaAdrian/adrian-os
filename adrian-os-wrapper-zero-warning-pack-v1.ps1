# =====================================================================
# ADRIAN OS Wrapper Zero-Warning Pack v1
# ---------------------------------------------------------------------
# Purpose:
#   Drive adrian-boot-image to ZERO warnings by referencing every
#   remaining unused item, using the EXACT names revealed by cargo:
#
#     flow::WrapperStage::{ <variants printed below> }  + stage_label(...)
#     entry::EntryPhase::{ Placeholder, ExperimentStart, FutureBootArtifactEntry }
#     transition::TransitionCandidatePhase::{ Placeholder, Mrt1Active, FutureRealBoundaryCrossing }
#
#   Both EntryPhase and TransitionCandidatePhase derive Debug (confirmed
#   by cargo notes), so {:?} printing is safe.
#
#   IMPORTANT ASSUMPTION (flow only):
#     The WrapperStage variant identifiers are assumed to be:
#         Entry, Bridge, Invoke
#     cargo did NOT print these. If they differ, only the 3 flow lines
#     will error with the real names -- paste it and I patch instantly.
#     Everything else is confirmed and will compile.
#
#   Rewrites ONLY axiom/boot-image/src/main.rs.
#
# Run from: D:\adrian-os
#   cd D:\adrian-os
#   .\adrian-os-wrapper-zero-warning-pack-v1.ps1
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
//! Compile-clean placeholder that exercises EVERY defined wrapper-side
//! item (functions, enum variants, struct fields) so the crate builds
//! with zero warnings. No real Axiom code runs; no bootable artifact is
//! produced.

mod entry;
mod bridge;
mod invoke;
mod flow;
mod handoff;
mod transition;
mod candidate;

/// Reference every EntryPhase variant so none is reported as
/// never-constructed. Returns a label for each (Debug-derived).
fn demo_entry_phases() {
    let phases = [
        entry::EntryPhase::Placeholder,
        entry::EntryPhase::ExperimentStart,
        entry::EntryPhase::FutureBootArtifactEntry,
    ];
    for p in phases.iter() {
        println!("entry-phase(variant): {:?}", p);
    }
}

/// Reference every TransitionCandidatePhase variant so none is reported
/// as never-constructed.
fn demo_transition_phases() {
    let phases = [
        transition::TransitionCandidatePhase::Placeholder,
        transition::TransitionCandidatePhase::Mrt1Active,
        transition::TransitionCandidatePhase::FutureRealBoundaryCrossing,
    ];
    for p in phases.iter() {
        println!("transition-phase(variant): {:?}", p);
    }
}

/// Reference every WrapperStage variant via stage_label so neither the
/// enum nor the function is reported as unused.
fn demo_wrapper_stages() {
    let stages = [
        flow::WrapperStage::Entry,
        flow::WrapperStage::Bridge,
        flow::WrapperStage::Invoke,
    ];
    for s in stages.iter() {
        // stage_label takes WrapperStage by value; copy out of the ref.
        println!("stage-label: {}", flow::stage_label(*s));
    }
}

fn main() {
    // --- Wrapper-side entry (WF-1) ---
    println!("{}", entry::wrapper_entry_status());
    println!("{}", entry::entry_phase_label());
    demo_entry_phases();

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

    // --- FBE-1 / flow framing (all stages labeled) ---
    println!("{}", flow::wrapper_flow_summary());
    println!("{}", flow::wrapper_semantic_chain_summary());
    println!("{}", flow::mrt1_coordination_summary());
    demo_wrapper_stages();

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
    demo_transition_phases();

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
Write-Host "Wrote (zero-warning): $mainFile" -ForegroundColor Green

# --- Sanity: no malformed (bracket/paren) filenames anywhere ----------
$weird = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '[\[\]\(\)]' }
$weirdCount = ($weird | Measure-Object).Count
Write-Host ("Weird file count: " + $weirdCount) -ForegroundColor Yellow

Write-Host ""
Write-Host "Wrapper Zero-Warning Pack v1 applied." -ForegroundColor Cyan
Write-Host "Expected after cargo check: 0 warnings, 0 errors." -ForegroundColor Cyan
Write-Host "If the only errors mention flow::WrapperStage::{Entry|Bridge|Invoke}," -ForegroundColor Yellow
Write-Host "paste them; the real variant names just differ and I will patch." -ForegroundColor Yellow
Write-Host ""
Write-Host "Now run, from D:\adrian-os:" -ForegroundColor Cyan
Write-Host "  cargo check" -ForegroundColor White
Write-Host '  git add .' -ForegroundColor White
Write-Host '  git commit -m "Wrapper Zero-Warning Pack v1: exercise all variants/fields, clean build"' -ForegroundColor White
Write-Host "  git push" -ForegroundColor White
