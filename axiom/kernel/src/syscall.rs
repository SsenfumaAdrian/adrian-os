use crate::error::KernelError;

/// Minimal syscall numbers scaffold.
/// These values are placeholders and not stable ABI definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    Invalid = 0,
    ProcessCreate = 1,
    ThreadCreate = 2,
    ChannelCreate = 3,
    EventCreate = 4,
    HandleClose = 5,
}

/// Placeholder syscall dispatch result.
pub type SyscallResult<T> = Result<T, KernelError>;

pub fn dispatch_syscall(number: u64) -> SyscallResult<u64> {
    match number {
        0 => Err(KernelError::InvalidArgument),
        _ => Err(KernelError::NotSupported),
    }
}
