/// Core kernel error model placeholder.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    InvalidArgument,
    OutOfMemory,
    PermissionDenied,
    NotSupported,
    NotFound,
    Busy,
    /// The target object (a channel, eventually other closeable
    /// kernel objects) is closed and can no longer be operated on.
    /// Distinct from NotFound: the object existed and is known, it's
    /// just no longer usable.
    Closed,
    InternalFailure,
}
