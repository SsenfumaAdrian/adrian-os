/// Core kernel error model placeholder.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    InvalidArgument,
    OutOfMemory,
    PermissionDenied,
    NotSupported,
    NotFound,
    Busy,
    InternalFailure,
}
