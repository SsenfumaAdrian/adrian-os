/// FBE-1 wrapper flow stage WF-1: entry stage.
///
/// This module models the conceptual wrapper-side starting point for
/// the first runnable boot experiment path.
///
/// Important:
/// - this is still compile-clean and host-workflow-friendly
/// - it is not yet a true low-level boot entry symbol
/// - it marks the future wrapper-side control boundary

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPhase {
    Placeholder,
    ExperimentStart,
    FutureBootArtifactEntry,
}

pub fn wrapper_entry_status() -> &'static str {
    "FBE-1 WF-1: wrapper entry placeholder"
}

pub fn entry_phase() -> EntryPhase {
    EntryPhase::ExperimentStart
}

pub fn entry_phase_label() -> &'static str {
    match entry_phase() {
        EntryPhase::Placeholder => "entry-phase: placeholder",
        EntryPhase::ExperimentStart => "entry-phase: experiment-start",
        EntryPhase::FutureBootArtifactEntry => "entry-phase: future-boot-artifact-entry",
    }
}
