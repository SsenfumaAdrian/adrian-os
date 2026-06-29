mod bridge;
mod entry;
mod flow;
mod invoke;

fn main() {
    // ADRIAN OS FBE-1 wrapper-flow scaffold.
    //
    // Current conceptual sequence:
    // WF-1 -> entry
    // WF-2 -> bridge
    // WF-3 -> invoke
    //
    // Later stages should transition toward Axiom entry, marker emission,
    // and deterministic halt under a runnable experiment path.

    println!("{}", flow::wrapper_flow_summary());

    println!("{}", flow::stage_label(flow::WrapperStage::Entry));
    println!("{}", entry::wrapper_entry_status());
    println!("{}", entry::entry_phase_label());

    println!("{}", flow::stage_label(flow::WrapperStage::Bridge));
    println!("{}", bridge::bridge_status());
    println!("{}", bridge::bridge_phase_label());

    println!("{}", flow::stage_label(flow::WrapperStage::Invoke));
    println!("{}", invoke::invocation_status());
    println!("{}", invoke::invocation_phase_label());
}
