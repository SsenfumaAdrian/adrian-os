//! ADRIAN OS boot-image wrapper: MRT-1 transition candidate.
//!
//! Post-cohesion refinement: now that the wrapper-side FMH-1 summary
//! cohesion is complete (bridge + invoke + transition all reference the
//! synthetic handoff summary), the MRT-1 candidate is promoted from a
//! flat status string into a small STRUCTURED type.
//!
//! This is still experiment-only. It does NOT perform a real boundary
//! crossing, does NOT construct a real BootContext, and does NOT call
//! into Axiom. It only models the readiness state of a future crossing
//! so later packs have a concrete object to evolve.

/// Structured description of the MRT-1 transition candidate.
///
/// Each field is a placeholder-grade readiness flag or label describing
/// how close the wrapper-side path is to a real (still experiment-bound)
/// transition. No field implies production semantics.
#[derive(Clone, Copy)]
pub struct TransitionCandidate {
    /// Candidate model version.
    pub version: u32,
    /// Candidate label (stable identifier).
    pub label: &'static str,
    /// Whether the wrapper-side flow (entry -> bridge -> invoke) is
    /// conceptually wired. True once the placeholder flow exists.
    pub flow_wired: bool,
    /// Whether the synthetic handoff summary cohesion is complete.
    pub summary_cohesion: bool,
    /// Whether a real Axiom call is performed. MUST stay false until a
    /// real boundary-crossing pack intentionally flips it.
    pub real_axiom_call: bool,
    /// Whether a real BootContext is constructed. MUST stay false until
    /// a real BootContext construction pack intentionally flips it.
    pub real_boot_context: bool,
    /// Human-readable readiness phase.
    pub phase_label: &'static str,
}

impl TransitionCandidate {
    /// The current MRT-1 candidate after post-cohesion refinement.
    pub const fn mrt1_current() -> Self {
        TransitionCandidate {
            version: 1,
            label: "mrt-1",
            flow_wired: true,
            summary_cohesion: true,
            real_axiom_call: false,
            real_boot_context: false,
            phase_label: "post-cohesion: structured candidate, experiment-only",
        }
    }

    /// Readiness gate for the FIRST real boundary crossing.
    ///
    /// Returns true only when the wrapper-side prerequisites are met.
    /// Note: even when this returns true, no crossing happens here; a
    /// future pack must perform it explicitly and intentionally.
    pub const fn ready_for_real_crossing_gate(&self) -> bool {
        self.flow_wired && self.summary_cohesion
    }

    /// Whether the candidate is still strictly experiment-only.
    pub const fn is_experiment_only(&self) -> bool {
        !self.real_axiom_call && !self.real_boot_context
    }
}

/// Backwards-compatible flat status string (kept so existing callers and
/// docs that reference the candidate marker continue to read cleanly).
pub fn candidate_status() -> &'static str {
    "MRT-1: wrapper-side transition candidate placeholder (structured, post-cohesion)"
}

/// Short structured summary line for the candidate, for surfacing in
/// the placeholder binary output.
pub fn candidate_summary(candidate: &TransitionCandidate) -> &'static str {
    // Returned as a static label rather than formatted to keep this
    // no_std-friendly and allocation-free for future reuse.
    if candidate.ready_for_real_crossing_gate() && candidate.is_experiment_only() {
        "candidate-summary: mrt-1 | flow-wired=true | cohesion=true | crossing-gate=open | experiment-only=true"
    } else {
        "candidate-summary: mrt-1 | state-not-yet-ready (gate closed or no longer experiment-only)"
    }
}
