#![no_std]

pub mod arch;
pub mod boot;
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
pub mod syscall;
pub mod thread;

/// Top-level kernel initialization entry point.
/// This remains a scaffold while bring-up proceeds.
pub fn kernel_init() {
    init::early_kernel_init();
}
