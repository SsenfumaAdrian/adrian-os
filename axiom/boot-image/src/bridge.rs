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
