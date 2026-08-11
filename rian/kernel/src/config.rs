/// Kernel configuration constants and feature toggles.

pub const KERNEL_NAME: &str = "ADRIAN OS Rian";
pub const KERNEL_VERSION: &str = "0.1.0";

/// The eventual full-system target, once a real dynamic allocator
/// exists. Deliberately NOT the same number as
/// process::MAX_PROCESSES: that's a much smaller, fixed-capacity
/// bound for early bring-up (an array baked into the kernel binary,
/// no heap to grow into yet), documented there as bounded by this
/// constant rather than equal to it -- these represent different
/// things at different stages, not an inconsistency to reconcile by
/// making the numbers match.
pub const KERNEL_MAX_PROCESSES: usize = 4096;

/// See KERNEL_MAX_PROCESSES -- same relationship to
/// thread::MAX_THREADS.
pub const KERNEL_MAX_THREADS: usize = 16384;
