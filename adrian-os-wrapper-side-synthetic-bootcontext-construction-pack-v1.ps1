# =====================================================================
# ADRIAN OS Wrapper-Side Synthetic BootContext Construction Pack v1
# ---------------------------------------------------------------------
# Purpose:
#   Build a REAL artifact-shaped object on the wrapper side: a synthetic
#   BootContext whose field shape mirrors what Axiom's
#   kernel_entry(&BootContext) will eventually expect.
#
#   Field shape: Minimal v1
#     version: u32
#     arch_label: &'static str
#     memory_map_present: bool
#     serial_available: bool
#     experiment_mode: bool
#
#   Placement: axiom/boot-image/src/bridge.rs (the BootContext bridge).
#
# Guarantees:
#   - Compile-clean
#   - Synthetic only: NO real Axiom call, NO real firmware BootContext
#   - candidate.real_boot_context stays FALSE (this is synthetic, not real)
#   - Plain filenames only (no Markdown-link paths)
#   - Explicit directory creation before any Set-Content
#
# Run from: D:\adrian-os
#   cd D:\adrian-os
#   .\adrian-os-wrapper-side-synthetic-bootcontext-construction-pack-v1.ps1
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

$bridgeFile = Join-Path $bootImageSrc "bridge.rs"
$mainFile   = Join-Path $bootImageSrc "main.rs"
$docFile    = Join-Path $docsInterfaces "synthetic-bootcontext-construction-v1.md"

# ======================================================================
# bridge.rs
# ----------------------------------------------------------------------
# Adds a synthetic, artifact-shaped SyntheticBootContext type plus a
# validation function. Preserves the existing bridge status/relation
# strings so prior callers continue to read cleanly.
# ======================================================================
$bridgeContent = @'
//! ADRIAN OS boot-image wrapper: synthetic BootContext bridge.
//!
//! This module is the wrapper-side "BootContext bridge". With this pack
//! it constructs a REAL artifact-shaped object: `SyntheticBootContext`.
//!
//! IMPORTANT: this context is SYNTHETIC. It is NOT provided by firmware
//! and is NOT the real Axiom BootContext. No Axiom call is performed.
//! Its field shape deliberately mirrors what Axiom's
//! `kernel_entry(&BootContext)` is expected to consume, so the two stay
//! consistent and a future real crossing has a stable contract.
//!
//! Field shape: Minimal v1.

/// Synthetic, wrapper-side BootContext (Minimal v1 shape).
///
/// Mirrors the expected Axiom BootContext contract closely enough to be
/// a precursor, while remaining strictly experiment-only.
#[derive(Clone, Copy)]
pub struct SyntheticBootContext {
    /// BootContext contract version.
    pub version: u32,
    /// Target architecture label (e.g. "x86_64").
    pub arch_label: &'static str,
    /// Whether a memory map is (synthetically) considered present.
    pub memory_map_present: bool,
    /// Whether a serial backend is (synthetically) considered available.
    pub serial_available: bool,
    /// Experiment marker. MUST stay true while this is synthetic-only.
    pub experiment_mode: bool,
}

impl SyntheticBootContext {
    /// The default synthetic BootContext for the FBE-1 experiment path.
    pub const fn fbe1_default() -> Self {
        SyntheticBootContext {
            version: 1,
            arch_label: "x86_64",
            memory_map_present: true,
            serial_available: true,
            experiment_mode: true,
        }
    }

    /// Wrapper-side structural validation of the synthetic context.
    ///
    /// Returns true when the synthetic context is well-formed enough to
    /// be considered a valid precursor to a future real crossing.
    /// This does NOT validate real firmware data (there is none yet).
    pub const fn is_well_formed(&self) -> bool {
        self.version >= 1
            && !self.arch_label.is_empty()
            && self.memory_map_present
            && self.serial_available
            && self.experiment_mode
    }
}

/// High-level bridge status (preserved).
pub fn bridge_status() -> &'static str {
    "FBE-1 WF-2: synthetic BootContext bridge placeholder"
}

/// Bridge phase (preserved).
pub fn bridge_phase() -> &'static str {
    "bridge-phase: experiment-preparing"
}

/// Bridge-to-handoff relation (preserved).
pub fn bridge_to_handoff() -> &'static str {
    "bridge-to-handoff: synthetic wrapper-side preparation intent"
}

/// Bridge-to-summary relation (preserved FMH-1 alignment).
pub fn bridge_to_summary() -> &'static str {
    "bridge-to-summary: bridge aligns with the active FMH-1 synthetic handoff summary model"
}

/// New: a status line describing the constructed synthetic BootContext.
pub fn bridge_context_status(context: &SyntheticBootContext) -> &'static str {
    if context.is_well_formed() {
        "bridge-context: synthetic BootContext constructed | shape=minimal-v1 | well-formed=true | experiment-only=true"
    } else {
        "bridge-context: synthetic BootContext malformed (precursor not yet valid)"
    }
}
'@

Set-Content -Path $bridgeFile -Value $bridgeContent -Encoding UTF8
Write-Host "Wrote: $bridgeFile" -ForegroundColor Green

# ======================================================================
# main.rs
# ----------------------------------------------------------------------
# Constructs and surfaces the synthetic BootContext alongside the flow.
# ======================================================================
$mainContent = @'
//! ADRIAN OS boot-image wrapper: placeholder binary entry.
//!
//! Compile-clean placeholder surfacing the wrapper-side conceptual flow,
//! the FMH-1 summary cohesion, the structured MRT-1 candidate, and now
//! a synthetic artifact-shaped BootContext. It does not produce a
//! bootable artifact and does not call into Axiom.

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

    // Cross-check: synthetic BootContext well-formedness vs candidate gate.
    println!(
        "precursor-check: boot_context_well_formed={} crossing_gate_open={} (still experiment-only, no real crossing)",
        boot_context.is_well_formed(),
        cand.ready_for_real_crossing_gate()
    );
}
'@

Set-Content -Path $mainFile -Value $mainContent -Encoding UTF8
Write-Host "Wrote: $mainFile" -ForegroundColor Green

# ======================================================================
# docs/interfaces/synthetic-bootcontext-construction-v1.md
# ======================================================================
$docContent = @'
# Synthetic BootContext Construction v1

## Purpose
Construct the first REAL artifact-shaped object on the wrapper side: a
synthetic `SyntheticBootContext` whose field shape mirrors the expected
Axiom `BootContext` contract. This is the concrete precursor to the first
genuine wrapper -> Axiom boundary crossing.

## Field shape (Minimal v1)
| Field               | Type           | Meaning                                  |
|---------------------|----------------|------------------------------------------|
| version             | u32            | BootContext contract version             |
| arch_label          | &'static str   | Target architecture label ("x86_64")     |
| memory_map_present  | bool           | Memory map (synthetically) present       |
| serial_available    | bool           | Serial backend (synthetically) available |
| experiment_mode     | bool           | MUST be true while synthetic-only        |

## Placement
`axiom/boot-image/src/bridge.rs` — the wrapper-side BootContext bridge.

## Methods
- `SyntheticBootContext::fbe1_default()` -> const default for FBE-1
- `is_well_formed()` -> structural validity of the synthetic precursor

## Safety invariants
- This is SYNTHETIC. It is not firmware-provided and is not the real
  Axiom BootContext.
- NO Axiom call is performed.
- The candidate's `real_boot_context` flag stays FALSE: this synthetic
  construction does not count as a real BootContext.
- `experiment_mode` stays true.

## Contract consistency note
The Minimal v1 shape is the agreed contract between the wrapper and
Axiom. When Axiom's real `BootContext` is finalized, its first five
fields should match this shape (or this shape should be updated in
lockstep) so the future real crossing has a stable, shared layout.

## Guarantees
- Compile-clean (`cargo check` expected to pass)
- No behavior change beyond constructing/validating a synthetic object

## Status
- Phase: MRT-1 post-cohesion, synthetic precursor constructed
- Crossing gate: open but uncrossed
- Next: design the first real (still experiment-bounded) crossing that
  passes this synthetic context across the boundary toward Axiom
'@

Set-Content -Path $docFile -Value $docContent -Encoding UTF8
Write-Host "Wrote: $docFile" -ForegroundColor Green

# --- Sanity: no malformed (bracket/paren) filenames -------------------
$weird = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '[\[\]\(\)]' }
Write-Host ("Weird file count: " + ($weird | Measure-Object).Count) -ForegroundColor Yellow

# --- Next steps for the user ------------------------------------------
Write-Host ""
Write-Host "Wrapper-Side Synthetic BootContext Construction Pack v1 applied." -ForegroundColor Cyan
Write-Host "Now run, from D:\adrian-os:" -ForegroundColor Cyan
Write-Host "  cargo check" -ForegroundColor White
Write-Host '  git add .' -ForegroundColor White
Write-Host '  git commit -m "Wrapper-Side Synthetic BootContext Construction Pack v1: minimal-v1 synthetic BootContext"' -ForegroundColor White
Write-Host "  git push" -ForegroundColor White
