pub mod arm64;
pub mod x86_64;

/// Dispatch early architecture initialization.
///
/// For now we default to x86_64 bring-up because that is the
/// initial ADRIAN OS reference target.
pub fn early_arch_init() {
    x86_64::early_arch_init();
}
