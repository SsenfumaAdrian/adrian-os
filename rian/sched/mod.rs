/// Scheduler scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingClass {
    Idle,
    Normal,
    Background,
    RealtimePlanned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerState {
    pub initialized: bool,
}

impl SchedulerState {
    pub const fn new() -> Self {
        Self { initialized: false }
    }
}

pub fn early_sched_init() {
    // Planned:
    // - initialize scheduler state
    // - establish idle thread concept
    // - prepare timer-driven preemption integration
}
