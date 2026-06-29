mod bridge;
mod entry;
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

    println!("{}", entry::wrapper_entry_status());
    println!("{}", bridge::bridge_status());
    println!("{}", invoke::invocation_status());
}
