use crate::object::KernelObjectId;

/// Basic thread lifecycle placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Created,
    Runnable,
    Running,
    Blocked,
    Suspended,
    Terminating,
    Terminated,
    Faulted,
}

/// Simplified thread model scaffold.
#[derive(Debug, Clone, Copy)]
pub struct Thread {
    pub id: KernelObjectId,
    pub process_id: KernelObjectId,
    pub state: ThreadState,
}

impl Thread {
    pub const fn new(id: KernelObjectId, process_id: KernelObjectId) -> Self {
        Self {
            id,
            process_id,
            state: ThreadState::Created,
        }
    }
}
