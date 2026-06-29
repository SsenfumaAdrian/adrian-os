fn main() {
    // ADRIAN OS boot-image scaffold.
    //
    // This binary is currently a placeholder whose job is to mark the
    // future boundary between:
    // - boot artifact concerns
    // - Axiom kernel core logic
    //
    // It is not yet a real bootable kernel image path.

    println!("{}", boot_image_status_message());
}

fn boot_image_status_message() -> &'static str {
    "ADRIAN OS boot-image scaffold: compile-valid placeholder"
}
