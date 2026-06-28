#![no_std]

pub mod boot;
pub mod config;
pub mod error;
pub mod init;
pub mod object;
pub mod panic;
pub mod process;
pub mod syscall;
pub mod thread;

/// Top-level kernel initialization entry point.
/// This is a placeholder scaffold and not yet a full bootable path.
pub fn kernel_init() {
    init::early_kernel_init();
}
