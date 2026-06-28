#![no_std]

pub mod boot;
pub mod init;
pub mod object;
pub mod panic;
pub mod process;
pub mod syscall;
pub mod thread;

pub fn kernel_init() {
    init::early_kernel_init();
}
