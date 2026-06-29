mod bridge;
mod entry;
mod invoke;

fn main() {
    // ADRIAN OS boot-image scaffold.
    //
    // This binary is currently a placeholder whose job is to mark the
    // future boundary between:
    // - boot artifact concerns
    // - Axiom kernel core logic
    //
    // It is not yet a real bootable kernel image path.

    println!("{}", entry::wrapper_entry_status());
    println!("{}", bridge::bridge_status());
    println!("{}", invoke::invocation_status());
}
