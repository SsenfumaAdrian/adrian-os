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

/// Fixed-capacity thread registry. Array-backed for the same reason
/// as sched::RunQueue: no heap allocator exists yet, so no
/// alloc::collections structure is available to hold this instead.
pub struct ThreadTable<const CAPACITY: usize> {
    threads: [Option<Thread>; CAPACITY],
}

impl<const CAPACITY: usize> ThreadTable<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            threads: [None; CAPACITY],
        }
    }

    pub fn count(&self) -> usize {
        self.threads.iter().filter(|t| t.is_some()).count()
    }

    /// Create a thread in the `Created` state, owned by `process_id`.
    /// `None` if the table is already full. A freshly created thread
    /// is not yet runnable -- see `make_runnable` -- matching the
    /// state machine the original scaffold already defined (Created
    /// is distinct from Runnable, not a synonym for it).
    pub fn spawn(&mut self, process_id: KernelObjectId) -> Option<KernelObjectId> {
        let slot = self.threads.iter().position(|t| t.is_none())?;
        let id = crate::object::allocate_id();
        self.threads[slot] = Some(Thread::new(id, process_id));
        Some(id)
    }

    pub fn get(&self, id: KernelObjectId) -> Option<&Thread> {
        self.threads.iter().flatten().find(|thread| thread.id == id)
    }

    pub fn set_state(&mut self, id: KernelObjectId, state: ThreadState) -> bool {
        match self.threads.iter_mut().flatten().find(|thread| thread.id == id) {
            Some(thread) => {
                thread.state = state;
                true
            }
            None => false,
        }
    }
}

/// The kernel's thread registry. A single global instance, same
/// pattern as mm::BOOTSTRAP_ALLOCATOR and sched::READY_QUEUE.
pub const MAX_THREADS: usize = 64;
pub static THREAD_TABLE: crate::sync::SpinLock<ThreadTable<MAX_THREADS>> =
    crate::sync::SpinLock::new(ThreadTable::new());

/// Transition a thread to `Runnable` in `table` and add it to
/// `queue`. Generic over both capacities rather than reaching into
/// the real globals directly, specifically so this is testable
/// against small local instances -- the actual logic (state
/// transition, then enqueue) doesn't depend on which table or queue
/// it's operating on.
pub fn make_runnable<const T: usize, const Q: usize>(
    table: &mut ThreadTable<T>,
    queue: &mut crate::sched::RunQueue<Q>,
    id: KernelObjectId,
) -> bool {
    if !table.set_state(id, ThreadState::Runnable) {
        return false;
    }
    queue.enqueue(id)
}

/// Early thread bring-up step: creates the kernel's own bootstrap
/// thread inside `process_id` and makes it runnable -- the first time
/// anything real goes into the scheduler's ready queue, rather than
/// the queue just existing empty and untested against reality.
pub fn early_thread_init(process_id: KernelObjectId) {
    let id = THREAD_TABLE
        .lock()
        .spawn(process_id)
        .expect("thread table has zero capacity: a build-time bug, not a runtime condition");

    let mut table = THREAD_TABLE.lock();
    let mut queue = crate::sched::READY_QUEUE.lock();
    make_runnable(&mut table, &mut queue, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u64) -> KernelObjectId {
        KernelObjectId(n)
    }

    #[test]
    fn spawn_creates_thread_in_created_state() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let id = table.spawn(pid(1)).unwrap();
        let thread = table.get(id).unwrap();
        assert_eq!(thread.state, ThreadState::Created);
        assert_eq!(thread.process_id, pid(1));
    }

    #[test]
    fn spawn_assigns_unique_ids() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let a = table.spawn(pid(1)).unwrap();
        let b = table.spawn(pid(1)).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn spawn_fails_when_table_is_full() {
        let mut table: ThreadTable<2> = ThreadTable::new();
        assert!(table.spawn(pid(1)).is_some());
        assert!(table.spawn(pid(1)).is_some());
        assert!(table.spawn(pid(1)).is_none());
        assert_eq!(table.count(), 2);
    }

    #[test]
    fn set_state_updates_the_right_thread_only() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let a = table.spawn(pid(1)).unwrap();
        let b = table.spawn(pid(1)).unwrap();

        assert!(table.set_state(a, ThreadState::Blocked));
        assert_eq!(table.get(a).unwrap().state, ThreadState::Blocked);
        assert_eq!(table.get(b).unwrap().state, ThreadState::Created);
    }

    #[test]
    fn set_state_on_unknown_id_fails() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        assert!(!table.set_state(pid(999), ThreadState::Runnable));
    }

    #[test]
    fn make_runnable_transitions_state_and_enqueues() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<4> = crate::sched::RunQueue::new();
        let id = table.spawn(pid(1)).unwrap();

        assert!(make_runnable(&mut table, &mut queue, id));
        assert_eq!(table.get(id).unwrap().state, ThreadState::Runnable);
        assert_eq!(queue.dequeue(), Some(id));
    }

    #[test]
    fn make_runnable_fails_for_unknown_thread_and_does_not_enqueue() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<4> = crate::sched::RunQueue::new();

        assert!(!make_runnable(&mut table, &mut queue, pid(404)));
        assert!(queue.is_empty());
    }
}
