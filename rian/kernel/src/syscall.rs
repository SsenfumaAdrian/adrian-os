use crate::error::KernelError;
use crate::security::{is_authorized, CapabilityRights, SecurityLabel};

/// Minimal syscall numbers scaffold.
/// These values are placeholders and not stable ABI definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    Invalid = 0,
    ProcessCreate = 1,
    ThreadCreate = 2,
    ChannelCreate = 3,
    EventCreate = 4,
    HandleClose = 5,
}

impl SyscallNumber {
    /// `#[repr(u64)]` enums don't get a free `TryFrom` -- this is the
    /// hand-written equivalent, kept in sync with the discriminants
    /// above by the round-trip test in this module rather than by
    /// hoping nobody edits one without the other.
    pub const fn from_u64(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Invalid),
            1 => Some(Self::ProcessCreate),
            2 => Some(Self::ThreadCreate),
            3 => Some(Self::ChannelCreate),
            4 => Some(Self::EventCreate),
            5 => Some(Self::HandleClose),
            _ => None,
        }
    }

    /// The authorization policy for this syscall, or `None` if the
    /// number names no actual operation.
    ///
    /// `Invalid` deliberately has no policy rather than an
    /// impossible-to-satisfy one: authorization answers "may this
    /// caller do this thing", and `Invalid` is not a thing that can be
    /// done. Returning `None` keeps that an ABI-level rejection
    /// (`InvalidArgument`) instead of dressing it up as a permission
    /// failure.
    ///
    /// The specific rights and labels below are a first cut, not a
    /// settled ABI -- the same caveat `is_authorized` carries about
    /// being one conservative policy shape rather than the definitive
    /// Adrian OS security model. What each line encodes:
    ///
    /// - every real syscall requires a *classified* caller, so
    ///   `Unknown` (which ranks below `Application`) is denied
    ///   everything. Deny-by-default for anything the system can't
    ///   classify, matching `SecurityLabel::trust_rank`'s reasoning.
    /// - `ProcessCreate` is additionally restricted to
    ///   `SystemService`-or-better: creating a new protection domain is
    ///   a service-level act, so applications and driver hosts are
    ///   denied it even holding WRITE.
    /// - the create syscalls require WRITE (they bring new kernel state
    ///   into existence); `HandleClose` requires DESTROY (it takes
    ///   state away). Splitting those two means a caller can be given
    ///   the ability to create objects without the ability to tear
    ///   down handles it was handed.
    pub const fn policy(&self) -> Option<SyscallPolicy> {
        let (minimum_label, required_rights) = match self {
            Self::Invalid => return None,
            Self::ProcessCreate => (SecurityLabel::SystemService, CapabilityRights::WRITE),
            Self::ThreadCreate => (SecurityLabel::Application, CapabilityRights::WRITE),
            Self::ChannelCreate => (SecurityLabel::Application, CapabilityRights::WRITE),
            Self::EventCreate => (SecurityLabel::Application, CapabilityRights::WRITE),
            Self::HandleClose => (SecurityLabel::Application, CapabilityRights::DESTROY),
        };
        Some(SyscallPolicy {
            minimum_label,
            required_rights,
        })
    }
}

/// What a caller must satisfy to invoke a given syscall: the least
/// trusted label permitted, and the capability rights required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallPolicy {
    /// The least-trusted label allowed to make this call. Checked with
    /// `SecurityLabel::at_least_as_trusted_as`, so a *more* trusted
    /// caller always passes.
    pub minimum_label: SecurityLabel,
    pub required_rights: CapabilityRights,
}

/// Who is making a syscall.
///
/// There is no privilege-transition trap handler yet, so nothing
/// populates this from hardware state and there is no current-thread
/// concept to read it from -- it is an explicit parameter for the same
/// reason `arg0` is, rather than pretending to recover a caller
/// identity that nothing has established. When real traps and context
/// switching exist, this is the struct the trap entry path fills in;
/// the enforcement below does not have to change for that to happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallContext {
    pub label: SecurityLabel,
    pub rights: CapabilityRights,
}

impl SyscallContext {
    pub const fn new(label: SecurityLabel, rights: CapabilityRights) -> Self {
        Self { label, rights }
    }

    /// The kernel calling itself: maximum trust, every right. This is
    /// what the parameterless `dispatch_syscall` uses, which is why
    /// wiring enforcement in did not change any existing caller's
    /// behaviour -- the kernel context passes every policy by
    /// construction.
    pub const fn kernel() -> Self {
        Self::new(SecurityLabel::Kernel, CapabilityRights::ALL)
    }

    /// `Ok(())` if this context satisfies `policy`, else
    /// `PermissionDenied`. Both conditions come from
    /// `security::is_authorized`; this function deliberately does not
    /// re-implement either half of that check.
    pub const fn authorize(&self, policy: SyscallPolicy) -> SyscallResult<()> {
        if is_authorized(
            self.label,
            self.rights,
            policy.minimum_label,
            policy.required_rights,
        ) {
            Ok(())
        } else {
            Err(KernelError::PermissionDenied)
        }
    }
}

/// Placeholder syscall dispatch result.
pub type SyscallResult<T> = Result<T, KernelError>;

/// Dispatch a syscall by number as the kernel itself.
///
/// Retained as the existing entry point and behaviourally unchanged:
/// `SyscallContext::kernel()` satisfies every policy, so nothing that
/// worked before this became an authorized path stops working. Any
/// caller that is *not* the kernel should use `dispatch_syscall_as` and
/// pass its real context.
pub fn dispatch_syscall(number: u64, arg0: u64) -> SyscallResult<u64> {
    dispatch_syscall_as(SyscallContext::kernel(), number, arg0)
}

/// Dispatch a syscall on behalf of `context`, enforcing that context's
/// label and capability rights against the syscall's policy before any
/// work happens.
///
/// `arg0`'s meaning depends on the syscall (e.g. for ThreadCreate, it's
/// the owning process's id) -- there's no real calling convention yet
/// (that needs an actual privilege-transition trap handler, not
/// attempted here), so this takes explicit typed arguments rather than
/// pretending to read registers that nothing has populated.
///
/// Order of checks, and why: the number is resolved first (an
/// unrecognized number is an ABI error, not a permission error), then
/// authorization, then the operation. Authorization strictly precedes
/// every side effect -- a denied call must not allocate an id, touch a
/// table, or consume a handle, so there is nothing to roll back.
pub fn dispatch_syscall_as(
    context: SyscallContext,
    number: u64,
    arg0: u64,
) -> SyscallResult<u64> {
    let syscall = SyscallNumber::from_u64(number).ok_or(KernelError::InvalidArgument)?;
    let policy = syscall.policy().ok_or(KernelError::InvalidArgument)?;
    context.authorize(policy)?;

    match syscall {
        // Unreachable now: `Invalid` has no policy, so it is rejected
        // above. Kept rather than replaced with `unreachable!()` so
        // that an accidental future edit giving Invalid a policy fails
        // closed here instead of becoming a live syscall.
        SyscallNumber::Invalid => Err(KernelError::InvalidArgument),

        SyscallNumber::ProcessCreate => crate::process::create_process().map(|id| id.0),

        SyscallNumber::ThreadCreate => {
            let process_id = crate::object::KernelObjectId(arg0);
            crate::thread::create_thread(process_id).map(|id| id.0)
        }

        // The handle registry (object.rs) now resolves what was
        // previously a real gap here -- ChannelCreate and HandleClose
        // can look up any id's kind and dispatch to the right table.
        SyscallNumber::ChannelCreate => crate::ipc::create_channel().ok_or(KernelError::OutOfMemory).map(|id| id.0),

        // EventObject now has real signal/clear/is_signaled logic and
        // a real table (ipc.rs), same as Channel got this same
        // commit.
        SyscallNumber::EventCreate => crate::ipc::create_event().ok_or(KernelError::OutOfMemory).map(|id| id.0),

        // Now a real generic dispatch: look up what kind of object
        // this id actually is via the handle registry, then destroy
        // it in whichever table actually owns it. Covers everything
        // with both a table and a registry-aware remove/destroy
        // method (Process, Thread, Channel); Event/Timer/etc. don't
        // have one yet, so those still report NotSupported rather
        // than silently succeeding at nothing.
        SyscallNumber::HandleClose => {
            let id = crate::object::KernelObjectId(arg0);
            // Bound to a variable, not used directly as the match
            // scrutinee: `match EXPR.lock().method() { ... }` keeps
            // EXPR's temporary guard alive for the whole match block,
            // not just this line, via Rust's temporary lifetime
            // extension for match scrutinees. The arms below call
            // into destroy_process/destroy_thread/destroy_channel,
            // each of which locks HANDLE_REGISTRY again internally --
            // holding it here too would self-deadlock against the
            // non-reentrant SpinLock, spinning forever rather than
            // failing loudly. Binding first drops the guard at the
            // end of this statement, before the match runs at all.
            let kind = crate::object::HANDLE_REGISTRY.lock().kind_of(id);
            match kind {
                Some(crate::object::KernelObjectKind::Process) => {
                    if crate::process::destroy_process(id) {
                        Ok(0)
                    } else {
                        Err(KernelError::NotFound)
                    }
                }
                Some(crate::object::KernelObjectKind::Thread) => {
                    if crate::thread::destroy_thread(id) {
                        Ok(0)
                    } else {
                        Err(KernelError::NotFound)
                    }
                }
                Some(crate::object::KernelObjectKind::Channel) => {
                    if crate::ipc::destroy_channel(id) {
                        Ok(0)
                    } else {
                        Err(KernelError::NotFound)
                    }
                }
                Some(crate::object::KernelObjectKind::Event) => {
                    if crate::ipc::destroy_event(id) {
                        Ok(0)
                    } else {
                        Err(KernelError::NotFound)
                    }
                }
                Some(_) => Err(KernelError::NotSupported),
                None => Err(KernelError::NotFound),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u64_round_trips_every_defined_number() {
        assert_eq!(SyscallNumber::from_u64(0), Some(SyscallNumber::Invalid));
        assert_eq!(SyscallNumber::from_u64(1), Some(SyscallNumber::ProcessCreate));
        assert_eq!(SyscallNumber::from_u64(2), Some(SyscallNumber::ThreadCreate));
        assert_eq!(SyscallNumber::from_u64(3), Some(SyscallNumber::ChannelCreate));
        assert_eq!(SyscallNumber::from_u64(4), Some(SyscallNumber::EventCreate));
        assert_eq!(SyscallNumber::from_u64(5), Some(SyscallNumber::HandleClose));
    }

    #[test]
    fn from_u64_rejects_undefined_numbers() {
        assert_eq!(SyscallNumber::from_u64(6), None);
        assert_eq!(SyscallNumber::from_u64(u64::MAX), None);
    }

    #[test]
    fn dispatching_an_undefined_number_is_rejected() {
        assert_eq!(dispatch_syscall(999, 0), Err(KernelError::InvalidArgument));
    }

    #[test]
    fn dispatching_invalid_is_rejected() {
        assert_eq!(dispatch_syscall(0, 0), Err(KernelError::InvalidArgument));
    }

    #[test]
    fn process_create_returns_a_valid_nonzero_id() {
        // Touches the real global PROCESS_TABLE via process::
        // create_process, same as any other test exercising this
        // syscall concurrently -- only asserting a relative property
        // (not the reserved sentinel 0) rather than anything about
        // total table state, so this stays safe regardless of test
        // execution order.
        let result = dispatch_syscall(SyscallNumber::ProcessCreate as u64, 0);
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn thread_create_returns_a_valid_nonzero_id_for_a_given_process() {
        let process_id = dispatch_syscall(SyscallNumber::ProcessCreate as u64, 0).unwrap();
        let thread_id = dispatch_syscall(SyscallNumber::ThreadCreate as u64, process_id);
        assert!(thread_id.is_ok());
        assert_ne!(thread_id.unwrap(), 0);
    }

    #[test]
    fn event_create_returns_a_valid_nonzero_id() {
        let result = dispatch_syscall(SyscallNumber::EventCreate as u64, 0);
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn handle_close_round_trips_a_real_event() {
        let event_id = dispatch_syscall(SyscallNumber::EventCreate as u64, 0).unwrap();
        assert_eq!(dispatch_syscall(SyscallNumber::HandleClose as u64, event_id), Ok(0));
        assert_eq!(
            dispatch_syscall(SyscallNumber::HandleClose as u64, event_id),
            Err(KernelError::NotFound)
        );
    }

    #[test]
    fn channel_create_returns_a_valid_nonzero_id() {
        let result = dispatch_syscall(SyscallNumber::ChannelCreate as u64, 0);
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn handle_close_on_unknown_id_is_not_found() {
        assert_eq!(
            dispatch_syscall(SyscallNumber::HandleClose as u64, u64::MAX),
            Err(KernelError::NotFound)
        );
    }

    #[test]
    fn handle_close_round_trips_a_real_process() {
        let process_id = dispatch_syscall(SyscallNumber::ProcessCreate as u64, 0).unwrap();
        assert_eq!(dispatch_syscall(SyscallNumber::HandleClose as u64, process_id), Ok(0));
        // Already closed -- the id is gone from the registry now.
        assert_eq!(
            dispatch_syscall(SyscallNumber::HandleClose as u64, process_id),
            Err(KernelError::NotFound)
        );
    }

    #[test]
    fn handle_close_round_trips_a_real_channel() {
        let channel_id = dispatch_syscall(SyscallNumber::ChannelCreate as u64, 0).unwrap();
        assert_eq!(dispatch_syscall(SyscallNumber::HandleClose as u64, channel_id), Ok(0));
        assert_eq!(
            dispatch_syscall(SyscallNumber::HandleClose as u64, channel_id),
            Err(KernelError::NotFound)
        );
    }

    // ---- capability enforcement -------------------------------------

    const REAL_SYSCALLS: [SyscallNumber; 5] = [
        SyscallNumber::ProcessCreate,
        SyscallNumber::ThreadCreate,
        SyscallNumber::ChannelCreate,
        SyscallNumber::EventCreate,
        SyscallNumber::HandleClose,
    ];

    #[test]
    fn every_real_syscall_has_a_policy_and_invalid_has_none() {
        for syscall in REAL_SYSCALLS {
            assert!(syscall.policy().is_some(), "{syscall:?} has no policy");
        }
        assert!(SyscallNumber::Invalid.policy().is_none());
    }

    #[test]
    fn an_unclassified_caller_is_denied_every_syscall() {
        // Unknown holding ALL rights: the rights half of the check
        // passes, so this isolates the label half. Deny-by-default for
        // anything the system can't classify.
        let unknown = SyscallContext::new(SecurityLabel::Unknown, CapabilityRights::ALL);
        for syscall in REAL_SYSCALLS {
            assert_eq!(
                dispatch_syscall_as(unknown, syscall as u64, 0),
                Err(KernelError::PermissionDenied),
                "{syscall:?} was not denied"
            );
        }
    }

    #[test]
    fn process_create_denies_an_application_even_with_every_right() {
        // The security-critical negative, now at the dispatch layer
        // rather than only in security.rs's own unit tests: holding the
        // right capability bits does not buy past a trust deficit.
        let app = SyscallContext::new(SecurityLabel::Application, CapabilityRights::ALL);
        assert_eq!(
            dispatch_syscall_as(app, SyscallNumber::ProcessCreate as u64, 0),
            Err(KernelError::PermissionDenied)
        );
    }

    #[test]
    fn process_create_allows_a_system_service_holding_write() {
        let service = SyscallContext::new(SecurityLabel::SystemService, CapabilityRights::WRITE);
        let result = dispatch_syscall_as(service, SyscallNumber::ProcessCreate as u64, 0);
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn an_application_holding_write_may_create_threads_channels_and_events() {
        let app = SyscallContext::new(SecurityLabel::Application, CapabilityRights::WRITE);
        let process_id = dispatch_syscall(SyscallNumber::ProcessCreate as u64, 0).unwrap();

        for (syscall, arg0) in [
            (SyscallNumber::ThreadCreate, process_id),
            (SyscallNumber::ChannelCreate, 0),
            (SyscallNumber::EventCreate, 0),
        ] {
            let result = dispatch_syscall_as(app, syscall as u64, arg0);
            assert!(result.is_ok(), "{syscall:?} was denied: {result:?}");
            assert_ne!(result.unwrap(), 0);
        }
    }

    #[test]
    fn a_caller_without_write_is_denied_the_create_syscalls() {
        // The mirror negative: ample trust, missing the one right the
        // policy asks for. PlatformService is the second-most-trusted
        // label there is and it still cannot create without WRITE.
        let read_only =
            SyscallContext::new(SecurityLabel::PlatformService, CapabilityRights::READ);
        for syscall in [
            SyscallNumber::ProcessCreate,
            SyscallNumber::ThreadCreate,
            SyscallNumber::ChannelCreate,
            SyscallNumber::EventCreate,
        ] {
            assert_eq!(
                dispatch_syscall_as(read_only, syscall as u64, 0),
                Err(KernelError::PermissionDenied),
                "{syscall:?} was not denied"
            );
        }
    }

    #[test]
    fn handle_close_requires_destroy_and_a_denial_leaves_the_object_intact() {
        // WRITE was enough to create; it is deliberately not enough to
        // close. The second half is the part worth asserting: after the
        // denial the handle must still be there, because authorization
        // runs before any side effect rather than being undone after
        // one.
        let creator = SyscallContext::new(SecurityLabel::Application, CapabilityRights::WRITE);
        let event_id = dispatch_syscall_as(creator, SyscallNumber::EventCreate as u64, 0).unwrap();

        assert_eq!(
            dispatch_syscall_as(creator, SyscallNumber::HandleClose as u64, event_id),
            Err(KernelError::PermissionDenied)
        );

        let closer = SyscallContext::new(SecurityLabel::Application, CapabilityRights::DESTROY);
        assert_eq!(
            dispatch_syscall_as(closer, SyscallNumber::HandleClose as u64, event_id),
            Ok(0)
        );
    }

    #[test]
    fn an_undefined_number_is_an_abi_error_not_a_permission_error() {
        // Even for a caller that would be denied everything: an
        // unrecognized number names no operation, so there is no
        // permission question to answer about it.
        let unknown = SyscallContext::new(SecurityLabel::Unknown, CapabilityRights::NONE);
        assert_eq!(
            dispatch_syscall_as(unknown, 999, 0),
            Err(KernelError::InvalidArgument)
        );
        assert_eq!(
            dispatch_syscall_as(unknown, SyscallNumber::Invalid as u64, 0),
            Err(KernelError::InvalidArgument)
        );
    }

    #[test]
    fn the_kernel_context_satisfies_every_policy() {
        let kernel = SyscallContext::kernel();
        for syscall in REAL_SYSCALLS {
            assert_eq!(kernel.authorize(syscall.policy().unwrap()), Ok(()));
        }
    }
}
