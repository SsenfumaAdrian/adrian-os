//! Structured boot progress tracking.
//!
//! The kernel's init sequence used to be observable only as a stream of
//! `&str` markers on the serial port. That is fine for a human watching
//! a terminal and useless for anything else: nothing could assert that
//! the sequence ran, ran in order, or ran to completion, so nine
//! subsystem init steps had zero test coverage.
//!
//! This module makes boot progress a value instead of a side effect.
//! Stages are recorded into a fixed-capacity array behind a lock, in
//! the order they were reached, and can be read back afterwards - by a
//! test, by the hosted dev-loop wrapper, or eventually by a diagnostic
//! syscall.
//!
//! Deliberately no allocation and no formatting machinery: this has to
//! work in the first microseconds of boot, before there is a heap, and
//! it must not be able to fail in a way that takes the boot down with
//! it. Overflow past `MAX_STAGES` is counted, not panicked on.

use crate::sync::SpinLock;

/// A named point in the kernel's initialization sequence.
///
/// Ordering of the variants is the intended ordering of the boot, which
/// is what makes `BootTrace::is_ordered` meaningful. `Idle` is last: it
/// means initialization finished and the kernel is ready to hand control
/// to whatever comes next (a scheduler, eventually; a halt, for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BootStage {
    Entry = 0,
    BootContextValidated = 1,
    ArchInit = 2,
    MemoryInit = 3,
    SecurityInit = 4,
    IpcInit = 5,
    SchedulerInit = 6,
    ProcessInit = 7,
    ThreadInit = 8,
    Idle = 9,
}

impl BootStage {
    /// Every stage, in the order a healthy boot reaches them.
    pub const ALL: [BootStage; 10] = [
        BootStage::Entry,
        BootStage::BootContextValidated,
        BootStage::ArchInit,
        BootStage::MemoryInit,
        BootStage::SecurityInit,
        BootStage::IpcInit,
        BootStage::SchedulerInit,
        BootStage::ProcessInit,
        BootStage::ThreadInit,
        BootStage::Idle,
    ];

    /// A short, stable identifier for logs. Kept as a `&'static str`
    /// rather than derived from `Debug` so the wire format of the boot
    /// log does not change if a variant is ever renamed.
    pub const fn label(&self) -> &'static str {
        match self {
            BootStage::Entry => "entry",
            BootStage::BootContextValidated => "boot-context",
            BootStage::ArchInit => "arch",
            BootStage::MemoryInit => "memory",
            BootStage::SecurityInit => "security",
            BootStage::IpcInit => "ipc",
            BootStage::SchedulerInit => "scheduler",
            BootStage::ProcessInit => "process",
            BootStage::ThreadInit => "thread",
            BootStage::Idle => "idle",
        }
    }
}

/// How many stages a trace can hold. Sized to `BootStage::ALL` with no
/// slack on purpose: a boot that records more stages than there are
/// stages is a bug (a double-record, or a re-entered init), and
/// `overflowed` is how that becomes visible instead of silently fitting.
pub const MAX_STAGES: usize = BootStage::ALL.len();

/// An ordered record of the stages boot has reached.
///
/// `Copy` and small (11 bytes plus padding), so it can be read out of
/// the global under the lock and then examined at leisure without
/// holding it - which matters because the natural caller is a debug
/// path, and debug paths must not be able to deadlock the thing they
/// are reporting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootTrace {
    stages: [Option<BootStage>; MAX_STAGES],
    len: usize,
    overflowed: bool,
}

impl BootTrace {
    pub const fn new() -> Self {
        Self {
            stages: [None; MAX_STAGES],
            len: 0,
            overflowed: false,
        }
    }

    /// Append `stage`. Returns `false` if the trace was already full,
    /// in which case the stage is dropped and `overflowed` latches.
    ///
    /// Never panics and never overwrites: losing a diagnostic is
    /// always preferable to taking down the boot that was trying to
    /// report it.
    pub fn record(&mut self, stage: BootStage) -> bool {
        if self.len >= MAX_STAGES {
            self.overflowed = true;
            return false;
        }
        self.stages[self.len] = Some(stage);
        self.len += 1;
        true
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn overflowed(&self) -> bool {
        self.overflowed
    }

    /// The stages reached, in the order they were reached.
    pub fn stages(&self) -> impl Iterator<Item = BootStage> + '_ {
        self.stages[..self.len].iter().filter_map(|s| *s)
    }

    pub fn last(&self) -> Option<BootStage> {
        if self.len == 0 {
            None
        } else {
            self.stages[self.len - 1]
        }
    }

    pub fn reached(&self, stage: BootStage) -> bool {
        self.stages().any(|s| s == stage)
    }

    /// Whether every recorded stage is strictly later than the one
    /// before it. Catches a stage recorded twice, or out of sequence,
    /// which a plain "did we reach it" check would miss.
    pub fn is_ordered(&self) -> bool {
        let mut previous: Option<BootStage> = None;
        for stage in self.stages() {
            if let Some(previous) = previous {
                if stage <= previous {
                    return false;
                }
            }
            previous = Some(stage);
        }
        true
    }

    /// A boot is complete when it reached every stage, in order, with
    /// nothing dropped.
    pub fn is_complete(&self) -> bool {
        self.len == MAX_STAGES && !self.overflowed && self.is_ordered()
    }
}

impl Default for BootTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// The kernel's boot trace.
///
/// A global rather than something threaded through every init function:
/// the whole point is that a subsystem can report progress without its
/// caller having to cooperate, and without changing nine signatures to
/// carry a recorder. Locked because an interrupt handler firing during
/// boot is a legitimate future recorder too.
pub static BOOT_TRACE: SpinLock<BootTrace> = SpinLock::new(BootTrace::new());

/// Record `stage` in the global trace and emit its label to the debug
/// output, in that order.
///
/// The trace write happens first deliberately: serial output is the
/// slower and more failure-prone of the two, and if it wedges we would
/// rather still have the record of how far boot got.
pub fn record(stage: BootStage) {
    BOOT_TRACE.lock().record(stage);
    crate::debug::debug_marker(stage.label());
}

/// A snapshot of the global trace. Copies out from under the lock.
pub fn snapshot() -> BootTrace {
    *BOOT_TRACE.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_trace_has_reached_nothing() {
        let trace = BootTrace::new();
        assert!(trace.is_empty());
        assert_eq!(trace.last(), None);
        assert!(!trace.overflowed());
        assert!(!trace.is_complete());
        // Vacuously ordered, which is the correct answer, not an
        // accident: there is no pair to be out of order.
        assert!(trace.is_ordered());
    }

    #[test]
    fn stages_come_back_in_the_order_they_were_recorded() {
        let mut trace = BootTrace::new();
        assert!(trace.record(BootStage::Entry));
        assert!(trace.record(BootStage::ArchInit));

        let seen: [BootStage; 2] = [
            trace.stages().next().unwrap(),
            trace.stages().nth(1).unwrap(),
        ];
        assert_eq!(seen, [BootStage::Entry, BootStage::ArchInit]);
        assert_eq!(trace.len(), 2);
        assert_eq!(trace.last(), Some(BootStage::ArchInit));
        assert!(trace.reached(BootStage::Entry));
        assert!(!trace.reached(BootStage::Idle));
    }

    #[test]
    fn the_full_documented_sequence_is_a_complete_boot() {
        let mut trace = BootTrace::new();
        for stage in BootStage::ALL {
            assert!(trace.record(stage), "{:?} should fit", stage);
        }
        assert_eq!(trace.len(), MAX_STAGES);
        assert!(trace.is_ordered());
        assert!(trace.is_complete());
        assert!(!trace.overflowed());
    }

    #[test]
    fn a_partial_boot_is_ordered_but_not_complete() {
        let mut trace = BootTrace::new();
        trace.record(BootStage::Entry);
        trace.record(BootStage::BootContextValidated);
        assert!(trace.is_ordered());
        assert!(
            !trace.is_complete(),
            "a boot that stopped after two stages must not report complete"
        );
    }

    #[test]
    fn out_of_order_and_repeated_stages_are_detected() {
        let mut backwards = BootTrace::new();
        backwards.record(BootStage::ArchInit);
        backwards.record(BootStage::Entry);
        assert!(!backwards.is_ordered(), "later-then-earlier must fail");

        let mut repeated = BootTrace::new();
        repeated.record(BootStage::Entry);
        repeated.record(BootStage::Entry);
        assert!(
            !repeated.is_ordered(),
            "the same stage twice is not forward progress"
        );
    }

    #[test]
    fn overflow_is_reported_rather_than_panicking_or_overwriting() {
        let mut trace = BootTrace::new();
        for stage in BootStage::ALL {
            trace.record(stage);
        }
        // One past capacity.
        assert!(!trace.record(BootStage::Entry));
        assert!(trace.overflowed());
        assert_eq!(trace.len(), MAX_STAGES, "capacity must not grow");
        assert_eq!(
            trace.last(),
            Some(BootStage::Idle),
            "the dropped stage must not overwrite the last good one"
        );
        assert!(
            !trace.is_complete(),
            "overflow means the record is untrustworthy, so not complete"
        );
    }

    #[test]
    fn labels_are_unique_so_a_boot_log_is_unambiguous() {
        let all = BootStage::ALL;
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a.label(), b.label(), "{:?} and {:?} share a label", a, b);
            }
        }
    }

    #[test]
    fn all_is_declared_in_ascending_order() {
        // ALL is the definition of "the intended boot order", and
        // is_ordered() is checked against the enum's own discriminants.
        // If those two ever disagree, every ordering assertion above
        // becomes meaningless, so pin them together here.
        for pair in BootStage::ALL.windows(2) {
            assert!(pair[0] < pair[1], "{:?} must precede {:?}", pair[0], pair[1]);
        }
    }
}

