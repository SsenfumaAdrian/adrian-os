/// Kernel object model placeholder.
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelObjectId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelObjectKind {
    Process,
    Thread,
    Channel,
    Event,
    Timer,
    SharedMemory,
    Device,
    AddressSpace,
    Unknown,
}

// Starts at 1, not 0: reserving KernelObjectId(0) as a sentinel for
// "no object" is a cheap, standard invariant to have (mirrors PID 0 /
// null-handle conventions elsewhere), and it's free to guarantee here
// by just not starting the counter there.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh, never-before-used id. A monotonic atomic counter
/// is deliberately all this is -- there's no contention pattern yet
/// that needs anything heavier, and a plain counter is trivially
/// correct to reason about. Revisit if that stops being true.
pub fn allocate_id() -> KernelObjectId {
    KernelObjectId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_id_returns_unique_monotonically_increasing_ids() {
        // NEXT_ID is a global shared across the whole test binary, so
        // this can't assert an absolute starting value -- other tests
        // may run first and advance it. What's always true regardless
        // of execution order: a sequence taken here is internally
        // consistent, strictly increasing, with no repeats.
        let a = allocate_id();
        let b = allocate_id();
        let c = allocate_id();

        assert!(b.0 > a.0);
        assert!(c.0 > b.0);
    }

    #[test]
    fn allocate_id_never_returns_the_reserved_sentinel() {
        assert_ne!(allocate_id().0, 0);
    }
}
