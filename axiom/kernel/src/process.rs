use crate::error::KernelError;
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

/// Fixed-capacity process registry. Array-backed for the same reason
/// as thread::ThreadTable and sched::RunQueue: no heap allocator
/// exists yet.
pub struct ProcessTable<const CAPACITY: usize> {
    processes: [Option<Process>; CAPACITY],
}

impl<const CAPACITY: usize> ProcessTable<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            processes: [None; CAPACITY],
        }
    }

    pub fn count(&self) -> usize {
        self.processes.iter().filter(|p| p.is_some()).count()
    }

    /// Create a process in the `Created` state. `None` if the table
    /// is already full. What `Initialized`/`Runnable` mean for a
    /// process beyond creation isn't defined anywhere yet, so this
    /// deliberately doesn't guess at driving further transitions --
    /// state changes past Created are left to whoever has that
    /// design worked out.
    pub fn spawn(&mut self) -> Option<KernelObjectId> {
        let slot = self.processes.iter().position(|p| p.is_none())?;
        let id = crate::object::allocate_id(crate::object::KernelObjectKind::Process)?;
        self.processes[slot] = Some(Process::new(id));
        Some(id)
    }

    pub fn get(&self, id: KernelObjectId) -> Option<&Process> {
        self.processes.iter().flatten().find(|process| process.id == id)
    }

    pub fn set_state(&mut self, id: KernelObjectId, state: ProcessState) -> bool {
        match self.processes.iter_mut().flatten().find(|process| process.id == id) {
            Some(process) => {
                process.state = state;
                true
            }
            None => false,
        }
    }
}

/// The kernel's process registry. A single global instance, same
/// pattern as mm::BOOTSTRAP_ALLOCATOR, sched::READY_QUEUE, and
/// thread::THREAD_TABLE.
pub const MAX_PROCESSES: usize = 32;
pub static PROCESS_TABLE: crate::sync::SpinLock<ProcessTable<MAX_PROCESSES>> =
    crate::sync::SpinLock::new(ProcessTable::new());

/// Create a new process in the `Created` state. This is the general
/// entry point -- both `early_process_init` (the one-time bootstrap
/// process) and, eventually, a real ProcessCreate syscall handler call
/// through here rather than duplicating table access.
pub fn create_process() -> Result<KernelObjectId, KernelError> {
    PROCESS_TABLE.lock().spawn().ok_or(KernelError::OutOfMemory)
}

/// Early process bring-up step: creates the kernel's own bootstrap
/// process -- the implicit process that owns kernel-mode execution
/// before any user process exists. Returns its id so
/// thread::early_thread_init can create a thread inside it.
pub fn early_process_init() -> KernelObjectId {
    create_process()
        .expect("process table has zero capacity: a build-time bug, not a runtime condition")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_creates_process_in_created_state() {
        let mut table: ProcessTable<4> = ProcessTable::new();
        let id = table.spawn().unwrap();
        assert_eq!(table.get(id).unwrap().state, ProcessState::Created);
    }

    #[test]
    fn spawn_assigns_unique_ids() {
        let mut table: ProcessTable<4> = ProcessTable::new();
        let a = table.spawn().unwrap();
        let b = table.spawn().unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn spawn_fails_when_table_is_full() {
        let mut table: ProcessTable<1> = ProcessTable::new();
        assert!(table.spawn().is_some());
        assert!(table.spawn().is_none());
        assert_eq!(table.count(), 1);
    }

    #[test]
    fn set_state_updates_the_right_process_only() {
        let mut table: ProcessTable<4> = ProcessTable::new();
        let a = table.spawn().unwrap();
        let b = table.spawn().unwrap();

        assert!(table.set_state(a, ProcessState::Crashed));
        assert_eq!(table.get(a).unwrap().state, ProcessState::Crashed);
        assert_eq!(table.get(b).unwrap().state, ProcessState::Created);
    }

    #[test]
    fn set_state_on_unknown_id_fails() {
        let mut table: ProcessTable<4> = ProcessTable::new();
        assert!(!table.set_state(KernelObjectId(999), ProcessState::Runnable));
    }
}
