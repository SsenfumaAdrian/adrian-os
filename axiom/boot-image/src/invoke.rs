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
