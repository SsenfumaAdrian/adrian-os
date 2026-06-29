/// Unified FBE-1 wrapper-side flow model.
///
/// This module provides a compile-clean representation of the
/// conceptual wrapper-side stage sequence:
/// - entry
/// - bridge
/// - invoke
///
/// It does not yet perform real runtime handoff behavior.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperStage {
    Entry,
    Bridge,
    Invoke,
}

pub fn stage_label(stage: WrapperStage) -> &'static str {
    match stage {
        WrapperStage::Entry => "wrapper-stage: entry",
        WrapperStage::Bridge => "wrapper-stage: bridge",
        WrapperStage::Invoke => "wrapper-stage: invoke",
    }
}

pub fn wrapper_flow_summary() -> &'static str {
    "FBE-1 wrapper flow: entry -> bridge -> invoke"
}
