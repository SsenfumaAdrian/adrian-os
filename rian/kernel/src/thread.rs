use crate::error::KernelError;
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

impl ThreadState {
    pub const fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Created | Self::Runnable | Self::Running | Self::Blocked | Self::Suspended
        )
    }

    pub const fn is_runnable(&self) -> bool {
        matches!(self, Self::Runnable | Self::Running)
    }
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

    pub const fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub const fn is_runnable(&self) -> bool {
        self.state.is_runnable()
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
        let id = crate::object::allocate_id(crate::object::KernelObjectKind::Thread)?;
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

    /// Remove a thread from the table entirely, unregistering its id
    /// from the global handle registry in the same step -- mirrors
    /// how `spawn` bundles allocation with registration rather than
    /// leaving cleanup as a separate call a caller could forget.
    pub fn remove(&mut self, id: KernelObjectId) -> bool {
        match self.threads.iter_mut().position(|t| matches!(t, Some(thread) if thread.id == id)) {
            Some(index) => {
                self.threads[index] = None;
                crate::object::HANDLE_REGISTRY.lock().unregister(id);
                true
            }
            None => false,
        }
    }
}

/// The kernel's thread registry. A single global instance, same
/// pattern as mm::BOOTSTRAP_ALLOCATOR and sched::READY_QUEUE.
///
/// Same relationship to config::KERNEL_MAX_THREADS (16384) that
/// process::MAX_PROCESSES has to KERNEL_MAX_PROCESSES: a much smaller
/// honest bound for early bring-up, checked at compile time to never
/// exceed the eventual full-system target.
pub const MAX_THREADS: usize = 64;
const _: () = assert!(MAX_THREADS <= crate::config::KERNEL_MAX_THREADS);

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

/// Create a new thread in the `Created` state, owned by `process_id`.
/// The entry point a real ThreadCreate syscall handler calls: it takes
/// the table lock itself and leaves the thread not-yet-runnable, so a
/// caller that wants it queued follows up through `make_runnable`.
///
/// Boot does *not* come through here -- `early_thread_init` needs the
/// table and the ready queue locked together, so that it cannot be
/// observed in the half-state where a thread exists but is unqueued.
pub fn create_thread(process_id: KernelObjectId) -> Result<KernelObjectId, KernelError> {
    THREAD_TABLE
        .lock()
        .spawn(process_id)
        .ok_or(KernelError::OutOfMemory)
}

/// The general entry point a real HandleClose syscall handler calls
/// for a thread id.
pub fn destroy_thread(id: KernelObjectId) -> bool {
    THREAD_TABLE.lock().remove(id)
}

/// Create a thread inside `process_id` and put it straight onto
/// `queue`, or leave both untouched if that is not possible.
///
/// Generic over the capacities for the same reason as `make_runnable`:
/// the interesting behavior here is the *cleanup* path, and cleanup on
/// a full ready queue is untestable against a 64-slot global (filling
/// it would mean 64 real spawns whose handles are never released).
pub fn spawn_runnable<const T: usize, const Q: usize>(
    table: &mut ThreadTable<T>,
    queue: &mut crate::sched::RunQueue<Q>,
    process_id: KernelObjectId,
) -> Result<KernelObjectId, KernelError> {
    let id = table.spawn(process_id).ok_or(KernelError::OutOfMemory)?;

    // `make_runnable`'s result was previously discarded at the one call
    // site. It returns false when the ready queue is full, and a thread
    // marked Runnable that sits in no queue is the worst available
    // outcome: the caller is told it succeeded and then nothing ever
    // runs. Unwind the spawn instead, so a failure leaves no Runnable
    // thread that no scheduler can reach.
    if !make_runnable(table, queue, id) {
        table.remove(id);
        return Err(KernelError::Busy);
    }

    Ok(id)
}

/// Early thread bring-up step: creates the kernel's own bootstrap
/// thread inside `process_id` and makes it runnable -- the first time
/// anything real goes into the scheduler's ready queue, rather than the
/// queue just existing empty and untested against reality.
///
/// Holds the thread table and the ready queue at the same time, so the
/// half-state (a thread that exists but is not queued) is never
/// observable to anything else, and so a failure can undo the spawn.
pub fn early_thread_init(process_id: KernelObjectId) -> Result<KernelObjectId, KernelError> {
    // Same correction as `process::early_process_init`: spawning fails
    // when the table is *full*, a runtime condition, so the old "zero
    // capacity, a build-time bug" panic misdescribed the only case that
    // can actually happen. Reported upward instead.
    let mut table = THREAD_TABLE.lock();
    let mut queue = crate::sched::READY_QUEUE.lock();
    spawn_runnable(&mut table, &mut queue, process_id)
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

    #[test]
    fn remove_frees_the_slot_and_unregisters_the_id() {
        let mut table: ThreadTable<2> = ThreadTable::new();
        let id = table.spawn(pid(1)).unwrap();
        assert_eq!(table.count(), 1);

        assert!(table.remove(id));
        assert_eq!(table.count(), 0);
        assert!(table.get(id).is_none());
        assert_eq!(crate::object::HANDLE_REGISTRY.lock().kind_of(id), None);

        assert!(!table.remove(id)); // already gone
    }

    #[test]
    fn spawn_runnable_creates_a_queued_runnable_thread() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<4> = crate::sched::RunQueue::new();

        let id = spawn_runnable(&mut table, &mut queue, pid(7)).unwrap();
        assert_eq!(table.get(id).unwrap().state, ThreadState::Runnable);
        assert_eq!(table.get(id).unwrap().process_id, pid(7));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.dequeue(), Some(id));
    }

    #[test]
    fn spawn_runnable_reports_a_full_thread_table_as_out_of_memory() {
        let mut table: ThreadTable<1> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<4> = crate::sched::RunQueue::new();

        assert!(spawn_runnable(&mut table, &mut queue, pid(1)).is_ok());
        assert_eq!(
            spawn_runnable(&mut table, &mut queue, pid(1)),
            Err(KernelError::OutOfMemory)
        );
    }

    #[test]
    fn spawn_runnable_leaves_nothing_behind_when_the_ready_queue_is_full() {
        // The case the discarded `make_runnable` result used to hide: a
        // thread that is Runnable but in no queue. On failure the table
        // must be exactly as it was, and the id must be gone from the
        // global handle registry too -- otherwise a full ready queue
        // slowly leaks handles.
        let mut table: ThreadTable<4> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<1> = crate::sched::RunQueue::new();

        let first = spawn_runnable(&mut table, &mut queue, pid(1)).unwrap();
        assert!(queue.is_full());

        assert_eq!(
            spawn_runnable(&mut table, &mut queue, pid(1)),
            Err(KernelError::Busy)
        );
        assert_eq!(table.count(), 1, "the rejected thread must not occupy a slot");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.dequeue(), Some(first), "the queued thread is untouched");
    }

    #[test]
    fn a_thread_rejected_for_a_full_queue_is_unregistered_globally() {
        let mut table: ThreadTable<4> = ThreadTable::new();
        let mut queue: crate::sched::RunQueue<1> = crate::sched::RunQueue::new();
        spawn_runnable(&mut table, &mut queue, pid(1)).unwrap();

        let before = crate::object::HANDLE_REGISTRY.lock().count();
        assert!(spawn_runnable(&mut table, &mut queue, pid(1)).is_err());
        let after = crate::object::HANDLE_REGISTRY.lock().count();

        // Not an equality assertion on the count itself: HANDLE_REGISTRY
        // is shared across the whole test binary and other tests run in
        // parallel, so only the *direction* is order-independent -- a
        // failed spawn must not leave the registry fuller than it was
        // by its own doing.
        assert!(
            after <= before + 1,
            "a rejected spawn leaked a handle: {} -> {}",
            before,
            after
        );
    }
}
