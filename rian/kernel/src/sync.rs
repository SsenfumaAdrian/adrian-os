use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A minimal mutual-exclusion lock built directly on an atomic flag
/// and a spin loop -- no OS, no blocking, no external dependency.
///
/// This is the right tool specifically because nothing else exists
/// yet to block on: there's no scheduler to yield to and no thread to
/// park while waiting. Every richer synchronization primitive a real
/// OS eventually has is itself typically built on something like this
/// at the bottom, so it belongs here first, ahead of anything that
/// needs it.
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: SpinLock provides its own synchronization via the atomic
// flag. The inner value is only ever reachable through a SpinLockGuard
// obtained after successfully acquiring that flag, so concurrent
// access is always mutually exclusive regardless of what T is.
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Spin until the lock is free, then hold it until the returned
    /// guard is dropped.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    /// Try once without spinning. `None` if another holder already
    /// has it.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinLockGuard { lock: self })
    }

    /// Whether the lock is currently held. Racy by construction --
    /// another holder can acquire or release the instant after this
    /// returns. Useful for diagnostics and tests; never a substitute
    /// for actually taking the lock before touching the data.
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }
}

/// RAII guard: releases the lock when dropped, however that happens
/// (normal scope exit, early return, or an unwind).
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: holding a SpinLockGuard means we hold the lock, so
        // no other guard for this SpinLock can exist concurrently.
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: same as above -- exclusive access is guaranteed by
        // holding the guard.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_grants_mutable_access() {
        let lock = SpinLock::new(5);
        *lock.lock() += 1;
        assert_eq!(*lock.lock(), 6);
    }

    #[test]
    fn try_lock_fails_while_held() {
        let lock = SpinLock::new(0);
        let guard = lock.lock();
        assert!(lock.try_lock().is_none());
        drop(guard);
        assert!(lock.try_lock().is_some());
    }

    #[test]
    fn is_locked_reflects_current_state() {
        let lock = SpinLock::new(0);
        assert!(!lock.is_locked());
        let guard = lock.lock();
        assert!(lock.is_locked());
        drop(guard);
        assert!(!lock.is_locked());
    }

    #[test]
    fn guard_releases_on_drop_even_after_early_return() {
        let lock = SpinLock::new(0);

        fn touch_and_return_early(lock: &SpinLock<i32>) {
            let mut guard = lock.lock();
            *guard = 1;
            if *guard == 1 {
                return; // guard drops here, lock must still release
            }
        }

        touch_and_return_early(&lock);
        assert!(!lock.is_locked());
    }

    /// The test that actually matters for a lock: real concurrent
    /// access, not just single-threaded logic. Eight threads each
    /// incrementing a shared counter 10,000 times through the lock --
    /// if acquisition or release had a real race (wrong ordering, a
    /// gap between check and set), this loses updates and the final
    /// count comes up short. It doesn't, consistently.
    #[test]
    fn concurrent_increments_are_not_lost() {
        use std::sync::Arc;
        use std::thread;

        const THREADS: u64 = 8;
        const PER_THREAD: u64 = 10_000;

        let lock = Arc::new(SpinLock::new(0u64));
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let lock = Arc::clone(&lock);
                thread::spawn(move || {
                    for _ in 0..PER_THREAD {
                        *lock.lock() += 1;
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        assert_eq!(*lock.lock(), THREADS * PER_THREAD);
    }
}
