use crate::object::KernelObjectId;

/// Basic process lifecycle placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Initialized,
    Runnable,
    Running,
    Blocked,
    Suspended,
    Terminating,
    Terminated,
    Crashed,
}

/// Simplified process model scaffold.
#[derive(Debug, Clone, Copy)]
pub struct Process {
    pub id: KernelObjectId,
    pub state: ProcessState,
}

impl Process {
    pub const fn new(id: KernelObjectId) -> Self {
        Self {
            id,
            state: ProcessState::Created,
        }
    }
}
