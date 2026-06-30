mod bridge;
mod entry;
mod flow;
mod handoff;
mod invoke;
mod transition;

fn main() {
    // ADRIAN OS FBE-1 wrapper-flow scaffold.
    //
    // Current conceptual sequence:
    // WF-1 -> entry
    // WF-2 -> bridge
    // WF-3 -> invoke
    //
    // Broader semantic chain:
    // entry -> bridge -> synthetic handoff -> invoke -> future Axiom entry

    let synthetic_handoff = handoff::SyntheticHandoff::fbe1_default();

    println!("{}", flow::wrapper_flow_summary());
    println!("{}", flow::wrapper_semantic_chain_summary());

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
}
