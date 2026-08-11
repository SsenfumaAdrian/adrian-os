use crate::error::KernelError;

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
}

/// Placeholder syscall dispatch result.
pub type SyscallResult<T> = Result<T, KernelError>;

/// Dispatch a syscall by number. `arg0`'s meaning depends on the
/// syscall (e.g. for ThreadCreate, it's the owning process's id) --
/// there's no real calling convention yet (that needs an actual
/// privilege-transition trap handler, not attempted here), so this
/// takes an explicit typed argument rather than pretending to read
/// registers that nothing has populated.
pub fn dispatch_syscall(number: u64, arg0: u64) -> SyscallResult<u64> {
    let syscall = SyscallNumber::from_u64(number).ok_or(KernelError::InvalidArgument)?;

    match syscall {
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
}
