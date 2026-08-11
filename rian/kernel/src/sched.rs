/// Scheduler ready-queue and ordering policy.
///
/// This module answers one question only: given a set of runnable
/// tasks, which one runs next. It deliberately does not touch task
/// lifecycle (creation, blocking, termination -- that's the
/// process/thread model, a separate roadmap step) or context
/// switching (register save/restore is architecture-specific machine
/// state that doesn't exist as a concept here yet). Keeping those
/// concerns apart means this module is fully testable without a real
/// task, a real stack, or real hardware -- exactly the same split
/// already used for mm's allocator and arch's IDT/PIC/PIT.
use crate::object::KernelObjectId;

/// Scheduling priority tiers. Not wired into RunQueue yet -- ordering
/// today is plain round-robin within a single queue, no tiers -- but
/// kept from the original scaffold as accurate forward-looking intent
/// rather than dropped for being unused right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingClass {
    Idle,
    Normal,
    Background,
    RealtimePlanned,
}

/// Coarse scheduler status flag, from the original scaffold. What
/// "initialized" means has shifted now that READY_QUEUE below is a
/// real static with real ready/enqueue/dequeue behavior rather than a
/// bare marker -- but the type itself isn't wrong, so it stays rather
/// than being removed on a judgment call nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerState {
    pub initialized: bool,
}

impl SchedulerState {
    pub const fn new() -> Self {
        Self { initialized: false }
    }
}

/// Ready-queue capacity for early bring-up. An honest reflection of
/// where the kernel actually is right now (no heap, so no
/// alloc::collections::VecDeque, so a fixed bound), not a permanent
/// ceiling -- revisit once a real kernel heap exists.
pub const MAX_TASKS: usize = 64;

/// Fixed-capacity FIFO ready queue: round-robin ordering, whoever's
/// been waiting longest runs next. Array-backed ring buffer rather
/// than a heap-allocated collection, since no heap allocator exists
/// yet -- no `#[global_allocator]` is set up anywhere in this crate.
pub struct RunQueue<const CAPACITY: usize> {
    tasks: [Option<KernelObjectId>; CAPACITY],
    head: usize,
    len: usize,
}

impl<const CAPACITY: usize> RunQueue<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            tasks: [None; CAPACITY],
            head: 0,
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == CAPACITY
    }

    /// Add a task to the back of the queue. `false` if the queue is
    /// already at capacity -- the caller decides what that means,
    /// there's no policy here for what to do about a full queue.
    pub fn enqueue(&mut self, task: KernelObjectId) -> bool {
        if self.is_full() {
            return false;
        }
        let tail = (self.head + self.len) % CAPACITY;
        self.tasks[tail] = Some(task);
        self.len += 1;
        true
    }

    /// Remove and return the task at the front of the queue -- the
    /// one that's been waiting longest. A task the caller wants to
    /// keep running (used its slice but is still runnable) goes back
    /// in with another `enqueue`, not handled automatically here:
    /// only the caller knows whether a task is still runnable versus
    /// now blocked or terminated.
    pub fn dequeue(&mut self) -> Option<KernelObjectId> {
        if self.is_empty() {
            return None;
        }
        let task = self.tasks[self.head].take();
        self.head = (self.head + 1) % CAPACITY;
        self.len -= 1;
        task
    }
}

/// The kernel's ready queue. A single global instance -- one CPU, one
/// scheduler, for now. Starts empty and stays empty until there's
/// real task creation (process/thread model, roadmap step 6, not yet
/// built) to enqueue something into it.
pub static READY_QUEUE: crate::sync::SpinLock<RunQueue<MAX_TASKS>> =
    crate::sync::SpinLock::new(RunQueue::new());

/// Early scheduler bring-up step.
///
/// Confirms the ready queue has a real static home and starts empty.
/// There's nothing to enqueue yet -- task creation (process/thread
/// model, roadmap step 6) doesn't exist, so an empty queue here is
/// accurate, not a placeholder standing in for something more real.
pub fn early_sched_init() {
    debug_assert!(READY_QUEUE.lock().is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_state_starts_uninitialized() {
        assert!(!SchedulerState::new().initialized);
    }

    #[test]
    fn new_queue_is_empty() {
        let queue: RunQueue<4> = RunQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn dequeue_on_empty_returns_none() {
        let mut queue: RunQueue<4> = RunQueue::new();
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn fifo_order_is_preserved() {
        let mut queue: RunQueue<4> = RunQueue::new();
        queue.enqueue(KernelObjectId(1));
        queue.enqueue(KernelObjectId(2));
        queue.enqueue(KernelObjectId(3));

        assert_eq!(queue.dequeue(), Some(KernelObjectId(1)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(2)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(3)));
    }

    #[test]
    fn enqueue_fails_when_full() {
        let mut queue: RunQueue<2> = RunQueue::new();
        assert!(queue.enqueue(KernelObjectId(1)));
        assert!(queue.enqueue(KernelObjectId(2)));
        assert!(queue.is_full());
        assert!(!queue.enqueue(KernelObjectId(3)));
        // A rejected enqueue must not corrupt what's already queued.
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn ring_buffer_wraps_around_correctly() {
        let mut queue: RunQueue<4> = RunQueue::new();

        for id in 1..=4u64 {
            assert!(queue.enqueue(KernelObjectId(id)));
        }
        assert!(queue.is_full());

        assert_eq!(queue.dequeue(), Some(KernelObjectId(1)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(2)));

        // Tail wraps past the array's physical end here: head=2,
        // len=2, so the next slot is index 0, not a new index past 3.
        assert!(queue.enqueue(KernelObjectId(5)));
        assert!(queue.enqueue(KernelObjectId(6)));
        assert!(queue.is_full());

        // FIFO order must hold across the wrap: 3 and 4 were enqueued
        // before 5 and 6, so they come out first despite 5/6 now
        // physically sitting earlier in the backing array.
        assert_eq!(queue.dequeue(), Some(KernelObjectId(3)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(4)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(5)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(6)));
        assert_eq!(queue.dequeue(), None);
        assert!(queue.is_empty());
    }

    #[test]
    fn requeueing_after_dequeue_moves_a_task_to_the_back() {
        // The round-robin pattern a caller actually uses: dequeue the
        // task to run, then -- if it's still runnable -- enqueue it
        // again rather than this doing that automatically.
        let mut queue: RunQueue<3> = RunQueue::new();
        queue.enqueue(KernelObjectId(1));
        queue.enqueue(KernelObjectId(2));

        let running = queue.dequeue().unwrap();
        assert_eq!(running, KernelObjectId(1));
        queue.enqueue(running); // still runnable, back of the line

        assert_eq!(queue.dequeue(), Some(KernelObjectId(2)));
        assert_eq!(queue.dequeue(), Some(KernelObjectId(1)));
    }

    #[test]
    fn global_ready_queue_starts_empty() {
        assert!(READY_QUEUE.lock().is_empty());
    }
}
