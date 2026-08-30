#![cfg_attr(not(feature = "std"), no_std)]

pub mod arch;
pub mod boot;
pub mod boot_trace;
pub mod config;
pub mod debug;
pub mod entry;
pub mod error;
pub mod init;
pub mod ipc;
pub mod mm;
pub mod object;
pub mod panic;
pub mod process;
pub mod sched;
pub mod security;
pub mod sync;
pub mod syscall;
pub mod thread;

// Deliberately no top-level init function here. The kernel's entry point
// is `entry::kernel_entry`, which `rian/boot-image` calls directly; it
// takes a `BootContext` and hands off to
// `init::early_kernel_init_with_context`.
//
// A `kernel_init()` wrapper used to sit at this spot with zero callers.
// Two entry points, only one of them real, is a trap for whoever reads
// `lib.rs` first -- removed rather than left as a decoy.
