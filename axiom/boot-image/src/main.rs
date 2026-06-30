mod bridge;
mod candidate;
mod entry;
mod flow;
mod handoff;
mod invoke;
mod transition;

fn main() {
    // ADRIAN OS FBE-1 wrapper-flow scaffold.

    let synthetic_handoff = handoff::SyntheticHandoff::fbe1_default();

    println!("{}", flow::wrapper_flow_summary());
    println!("{}", flow::wrapper_semantic_chain_summary());
    println!("{}", flow::mrt1_coordination_summary());

    println!("{}", candidate::candidate_status());
    println!("{}", candidate::candidate_label());

    println!("{}", flow::stage_label(flow::WrapperStage::Entry));
    println!("{}", entry::wrapper_entry_status());
    println!("{}", entry::entry_phase_label());

    println!("{}", flow::stage_label(flow::WrapperStage::Bridge));
    println!("{}", bridge::bridge_status());
    println!("{}", bridge::bridge_phase_label());
    println!("{}", bridge::bridge_handoff_relation());

    println!("{}", handoff::handoff_status());
    println!("{}", synthetic_handoff.status_label);

    println!("{}", flow::stage_label(flow::WrapperStage::Invoke));
    println!("{}", invoke::invocation_status());
    println!("{}", invoke::invocation_phase_label());
    println!("{}", invoke::invocation_handoff_relation());

    println!("{}", transition::transition_status());
    println!("{}", transition::transition_boundary_label());
    println!("{}", transition::transition_candidate_phase_label());
    println!("{}", transition::transition_marker_proof_relation());
}
