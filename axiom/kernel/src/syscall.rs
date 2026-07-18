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

        // Channels, events, and handle closing all need a unified
        // handle table -- one lookup covering processes, threads,
        // channels, and events together -- that doesn't exist yet.
        // ProcessTable and ThreadTable are each their own separate
        // store right now, which is fine for what created them
        // directly, but a syscall boundary needs one shared space to
        // hand ids back into. That's a real design question on its
        // own, not a quick bolt-on here -- Channel and Message
        // already have real send/receive logic (see ipc.rs), just no
        // syscall-reachable home yet.
        SyscallNumber::ChannelCreate | SyscallNumber::EventCreate | SyscallNumber::HandleClose => {
            Err(KernelError::NotSupported)
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
    fn channel_event_and_handle_close_are_not_yet_supported() {
        assert_eq!(
            dispatch_syscall(SyscallNumber::ChannelCreate as u64, 0),
            Err(KernelError::NotSupported)
        );
        assert_eq!(
            dispatch_syscall(SyscallNumber::EventCreate as u64, 0),
            Err(KernelError::NotSupported)
        );
        assert_eq!(
            dispatch_syscall(SyscallNumber::HandleClose as u64, 0),
            Err(KernelError::NotSupported)
        );
    }
}
