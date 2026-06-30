//! ADRIAN OS boot-image wrapper: placeholder binary entry.
//!
//! Compile-clean placeholder. This file calls ONLY functions that exist
//! in the current module files. It surfaces the wrapper-side conceptual
//! flow, the synthetic BootContext, the synthetic handoff, the
//! transition boundary, the MRT-1 candidate, and the first simulated
//! boundary crossing. No real Axiom code runs; no bootable artifact is
//! produced.

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

    // --- Structured MRT-1 transition candidate ---
    let cand = candidate::TransitionCandidate::mrt1_current();
    println!("{}", candidate::candidate_status());
    println!("{}", candidate::candidate_summary(&cand));
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
