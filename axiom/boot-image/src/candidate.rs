/// Minimal real transition candidate model for FBE-1.
///
/// This module represents MRT-1 in compile-clean wrapper-side code.
///
/// Important:
/// - experiment-oriented only
/// - not a production boot path
/// - not yet a real Axiom invocation
/// - intended to guide future marker-proof-oriented refinement

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MrtCandidate {
    Placeholder,
    Mrt1,
}

pub fn candidate_status() -> &'static str {
    "MRT-1: wrapper-side transition candidate placeholder"
}

pub fn candidate_label() -> &'static str {
    match current_candidate() {
        MrtCandidate::Placeholder => "candidate: placeholder",
        MrtCandidate::Mrt1 => "candidate: mrt-1",
    }
}

pub fn current_candidate() -> MrtCandidate {
    MrtCandidate::Mrt1
}
