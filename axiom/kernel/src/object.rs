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

/// A lightweight registry mapping id -> kind. Deliberately not a copy
/// of the object itself -- the real data stays in its own table
/// (process::PROCESS_TABLE, thread::THREAD_TABLE, and eventually
/// others); this only answers "what kind of thing is this id", which
/// is what a generic operation across object kinds (a real
/// HandleClose, eventually) needs to know which table to actually
/// operate on. This is exactly the "unified handle table" gap flagged
/// as missing when ChannelCreate/EventCreate/HandleClose were left
/// unsupported in syscall.rs.
pub struct HandleRegistry<const CAPACITY: usize> {
    entries: [Option<(KernelObjectId, KernelObjectKind)>; CAPACITY],
}

impl<const CAPACITY: usize> HandleRegistry<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
        }
    }

    pub fn count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// Private deliberately: registration should only ever happen as
    /// part of `allocate_id`, never on its own. A public `register`
    /// would let something register an id that wasn't actually handed
    /// out by the counter, which is exactly the inconsistent state
    /// this type exists to prevent.
    fn register(&mut self, id: KernelObjectId, kind: KernelObjectKind) -> bool {
        match self.entries.iter().position(|e| e.is_none()) {
            Some(slot) => {
                self.entries[slot] = Some((id, kind));
                true
            }
            None => false,
        }
    }

    pub fn kind_of(&self, id: KernelObjectId) -> Option<KernelObjectKind> {
        self.entries
            .iter()
            .flatten()
            .find(|(entry_id, _)| *entry_id == id)
            .map(|(_, kind)| *kind)
    }

    /// Remove an id from the registry -- called once the underlying
    /// object is actually destroyed. Doesn't touch the object's real
    /// table; that's the caller's responsibility, same division as
    /// registration (this only ever tracks kind, never owns data).
    pub fn unregister(&mut self, id: KernelObjectId) -> bool {
        match self
            .entries
            .iter_mut()
            .position(|e| matches!(e, Some((entry_id, _)) if *entry_id == id))
        {
            Some(slot) => {
                self.entries[slot] = None;
                true
            }
            None => false,
        }
    }
}

/// The kernel's handle registry. Generous capacity relative to the
/// current process/thread table sizes (32 + 64 = 96) since nothing
/// unregisters yet during normal operation, and it's shared across
/// this crate's entire test binary too -- every test that spawns a
/// process or thread through the real allocate_id path adds an entry
/// here that's never removed within that test run.
pub const MAX_HANDLES: usize = 256;
pub static HANDLE_REGISTRY: crate::sync::SpinLock<HandleRegistry<MAX_HANDLES>> =
    crate::sync::SpinLock::new(HandleRegistry::new());

/// Allocate a fresh, never-before-used id and register it as `kind` in
/// the global handle registry -- atomically from the caller's
/// perspective, so registration isn't a separate step a caller could
/// forget. `None` if the registry itself is full (the id counter never
/// runs out in any practical sense; the fixed-capacity registry is the
/// real limit).
pub fn allocate_id(kind: KernelObjectKind) -> Option<KernelObjectId> {
    let id = KernelObjectId(NEXT_ID.fetch_add(1, Ordering::Relaxed));
    if HANDLE_REGISTRY.lock().register(id, kind) {
        Some(id)
    } else {
        None
    }
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
        let a = allocate_id(KernelObjectKind::Unknown).unwrap();
        let b = allocate_id(KernelObjectKind::Unknown).unwrap();
        let c = allocate_id(KernelObjectKind::Unknown).unwrap();

        assert!(b.0 > a.0);
        assert!(c.0 > b.0);
    }

    #[test]
    fn allocate_id_never_returns_the_reserved_sentinel() {
        assert_ne!(allocate_id(KernelObjectKind::Unknown).unwrap().0, 0);
    }

    #[test]
    fn allocated_id_is_registered_with_the_correct_kind() {
        let id = allocate_id(KernelObjectKind::Channel).unwrap();
        assert_eq!(HANDLE_REGISTRY.lock().kind_of(id), Some(KernelObjectKind::Channel));
    }

    #[test]
    fn kind_of_an_id_nobody_allocated_returns_none() {
        // u64::MAX is astronomically unlikely to ever be reached by
        // the sequential counter, regardless of how many other tests
        // run first -- safe to assert against unconditionally.
        assert_eq!(HANDLE_REGISTRY.lock().kind_of(KernelObjectId(u64::MAX)), None);
    }

    #[test]
    fn unregister_removes_the_entry() {
        let id = allocate_id(KernelObjectKind::Event).unwrap();
        assert!(HANDLE_REGISTRY.lock().kind_of(id).is_some());
        assert!(HANDLE_REGISTRY.lock().unregister(id));
        assert_eq!(HANDLE_REGISTRY.lock().kind_of(id), None);
    }

    #[test]
    fn local_registry_respects_capacity() {
        let mut registry: HandleRegistry<2> = HandleRegistry::new();
        assert!(registry.register(KernelObjectId(100), KernelObjectKind::Process));
        assert!(registry.register(KernelObjectId(101), KernelObjectKind::Thread));
        assert!(!registry.register(KernelObjectId(102), KernelObjectKind::Channel));
        assert_eq!(registry.count(), 2);
    }

    #[test]
    fn local_registry_kind_of_and_unregister() {
        let mut registry: HandleRegistry<4> = HandleRegistry::new();
        let id = KernelObjectId(1);

        registry.register(id, KernelObjectKind::Timer);
        assert_eq!(registry.kind_of(id), Some(KernelObjectKind::Timer));

        assert!(registry.unregister(id));
        assert_eq!(registry.kind_of(id), None);
        assert!(!registry.unregister(id)); // already gone
    }
}
