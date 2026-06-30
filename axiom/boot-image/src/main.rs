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
