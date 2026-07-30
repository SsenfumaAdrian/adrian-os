//! Pulse: system initialization and service management.
//!
//! Split into one module per responsibility from Pulse's own README
//! (boot service graph + dependency resolution, restart policy,
//! lifecycle transitions, health supervision) -- this file used to
//! hold all four in one 543-line lib.rs; graphify's cohesion analysis
//! flagged it as doing too much in one place (0.063, the worst score
//! in the workspace), which matches how the kernel side already
//! splits arch/x86_64 into one file per concern (idt.rs, pic.rs,
//! pit.rs, paging.rs) rather than one giant arch file. Pure
//! reorganization -- every type, function, and test moved as-is, no
//! logic changed, re-verified against the exact same 30 tests
//! afterward.
//!
//! Same "decision, not execution" split throughout every module here:
//! none of this touches a real running service, since that needs
//! real process execution that doesn't exist yet on the kernel side
//! either (no context switching, no bare-metal target this sandbox
//! can compile for).

pub mod health;
pub mod lifecycle;
pub mod manifest;
pub mod restart;

pub use health::{HealthPolicy, HealthStatus};
pub use lifecycle::{is_valid_transition, ServiceState};
pub use manifest::{resolve_start_order, ResolutionError, ServiceManifest};
pub use restart::RestartPolicy;
