/// Kernel security hooks scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLabel {
    Kernel,
    PlatformService,
    SystemService,
    Application,
    DriverHost,
    Unknown,
}

impl SecurityLabel {
    /// A coarse trust ordering, most to least privileged. Unknown
    /// ranks below even Application deliberately: something the
    /// system can't classify gets treated with more suspicion than a
    /// known, presumably-vetted category, not less -- deny-by-default
    /// for anything unclassified.
    const fn trust_rank(&self) -> u8 {
        match self {
            Self::Kernel => 0,
            Self::PlatformService => 1,
            Self::SystemService => 2,
            Self::DriverHost => 3,
            Self::Application => 4,
            Self::Unknown => 5,
        }
    }

    /// Whether `self` is at least as trusted as `other`. A necessary
    /// -- not sufficient -- condition for `self` to grant capabilities
    /// scoped to `other`'s trust level; the actual capability rights
    /// involved still have to be checked separately via
    /// CapabilityRights::can_derive.
    pub const fn at_least_as_trusted_as(&self, other: Self) -> bool {
        self.trust_rank() <= other.trust_rank()
    }
}

/// A set of capability rights, bitflag-style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRights {
    bits: u64,
}

impl CapabilityRights {
    pub const NONE: Self = Self { bits: 0 };
    pub const READ: Self = Self { bits: 1 << 0 };
    pub const WRITE: Self = Self { bits: 1 << 1 };
    pub const EXECUTE: Self = Self { bits: 1 << 2 };
    pub const DUPLICATE: Self = Self { bits: 1 << 3 };
    pub const TRANSFER: Self = Self { bits: 1 << 4 };
    pub const DESTROY: Self = Self { bits: 1 << 5 };

    /// Every right defined above. The "full trust" baseline (the
    /// kernel's own capabilities, for instance) and an upper bound for
    /// validating that a requested set doesn't include undefined bits.
    pub const ALL: Self = Self {
        bits: Self::READ.bits
            | Self::WRITE.bits
            | Self::EXECUTE.bits
            | Self::DUPLICATE.bits
            | Self::TRANSFER.bits
            | Self::DESTROY.bits,
    };

    pub const fn contains(&self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    pub const fn union(&self, other: Self) -> Self {
        Self { bits: self.bits | other.bits }
    }

    pub const fn intersection(&self, other: Self) -> Self {
        Self { bits: self.bits & other.bits }
    }

    /// The rights in `self` with everything in `other` removed. Used
    /// to compute a narrowed capability -- e.g. a read-only duplicate
    /// of something that also had WRITE: `rights.without(Self::WRITE)`.
    pub const fn without(&self, other: Self) -> Self {
        Self { bits: self.bits & !other.bits }
    }

    pub const fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Whether `requested` can be derived from `self` by only removing
    /// rights, never adding any. This is the core capability
    /// invariant: a holder can narrow what they hand out, never widen
    /// it past what they themselves hold. Named and tested explicitly
    /// rather than left as something every call site has to remember
    /// to enforce on its own.
    pub const fn can_derive(&self, requested: Self) -> bool {
        self.contains(requested)
    }
}

/// Whether a holder is authorized to perform `requested_rights` on an
/// object labeled `target_label`, given the holder's own label and
/// rights. Two independent conditions, both required:
///
/// - the requested rights must be derivable from what the holder
///   actually has (`CapabilityRights::can_derive`)
/// - the holder's trust label must be at least as trusted as the
///   target's (`SecurityLabel::at_least_as_trusted_as`)
///
/// This is one reasonable, conservative policy shape -- deliberately
/// not claimed as the definitive Adrian OS security model, which is a
/// bigger design question than one function should settle on its own.
/// The conservative choice made here: a capability alone doesn't
/// bypass label-based trust policy, it only scopes what's possible
/// once trust policy already permits the interaction. An under-
/// trusted holder is denied even if it happens to hold the exact
/// right capability bits.
pub const fn is_authorized(
    holder_label: SecurityLabel,
    holder_rights: CapabilityRights,
    target_label: SecurityLabel,
    requested_rights: CapabilityRights,
) -> bool {
    holder_label.at_least_as_trusted_as(target_label) && holder_rights.can_derive(requested_rights)
}

pub fn early_security_init() {
    // CapabilityRights and SecurityLabel have real logic -- rights
    // composition, the can_derive invariant, a trust ordering on
    // labels, and is_authorized combining both into one check -- all
    // proven correct by this module's own tests.
    //
    // is_authorized is now actually enforced: syscall.rs gives every
    // syscall a SyscallPolicy (a minimum label plus required rights)
    // and dispatch_syscall_as checks the caller's SyscallContext
    // against it before performing any work, denying with
    // PermissionDenied. What is still missing is not enforcement but
    // *provenance*: nothing populates a SyscallContext from hardware
    // state, because there is no privilege-transition trap handler and
    // no current-thread concept to read a caller identity from, so the
    // context is passed explicitly by whoever calls dispatch. Kernel
    // objects also carry no label of their own yet -- the label half of
    // the check compares against the syscall's minimum, not against the
    // target object's own trust level.
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_LABELS: [SecurityLabel; 6] = [
        SecurityLabel::Kernel,
        SecurityLabel::PlatformService,
        SecurityLabel::SystemService,
        SecurityLabel::DriverHost,
        SecurityLabel::Application,
        SecurityLabel::Unknown,
    ];

    #[test]
    fn kernel_is_at_least_as_trusted_as_everything() {
        for label in ALL_LABELS {
            assert!(SecurityLabel::Kernel.at_least_as_trusted_as(label));
        }
    }

    #[test]
    fn unknown_is_at_least_as_trusted_as_only_itself() {
        for label in ALL_LABELS {
            let expected = label == SecurityLabel::Unknown;
            assert_eq!(SecurityLabel::Unknown.at_least_as_trusted_as(label), expected);
        }
    }

    #[test]
    fn every_label_is_at_least_as_trusted_as_itself() {
        for label in ALL_LABELS {
            assert!(label.at_least_as_trusted_as(label));
        }
    }

    #[test]
    fn none_is_empty_and_all_is_not() {
        assert!(CapabilityRights::NONE.is_empty());
        assert!(!CapabilityRights::ALL.is_empty());
    }

    #[test]
    fn all_contains_every_individual_right() {
        for right in [
            CapabilityRights::READ,
            CapabilityRights::WRITE,
            CapabilityRights::EXECUTE,
            CapabilityRights::DUPLICATE,
            CapabilityRights::TRANSFER,
            CapabilityRights::DESTROY,
        ] {
            assert!(CapabilityRights::ALL.contains(right));
        }
    }

    #[test]
    fn contains_is_reflexive_and_respects_none() {
        let rights = CapabilityRights::READ.union(CapabilityRights::WRITE);
        assert!(rights.contains(rights));
        assert!(rights.contains(CapabilityRights::NONE));
        assert!(!CapabilityRights::NONE.contains(rights));
    }

    #[test]
    fn union_combines_and_intersection_narrows() {
        let read_write = CapabilityRights::READ.union(CapabilityRights::WRITE);
        assert!(read_write.contains(CapabilityRights::READ));
        assert!(read_write.contains(CapabilityRights::WRITE));
        assert!(!read_write.contains(CapabilityRights::EXECUTE));

        let read_only = read_write.intersection(CapabilityRights::READ);
        assert!(read_only.contains(CapabilityRights::READ));
        assert!(!read_only.contains(CapabilityRights::WRITE));
    }

    #[test]
    fn without_removes_only_the_specified_rights() {
        let full = CapabilityRights::READ
            .union(CapabilityRights::WRITE)
            .union(CapabilityRights::DUPLICATE);
        let narrowed = full.without(CapabilityRights::WRITE);

        assert!(narrowed.contains(CapabilityRights::READ));
        assert!(narrowed.contains(CapabilityRights::DUPLICATE));
        assert!(!narrowed.contains(CapabilityRights::WRITE));
    }

    #[test]
    fn can_derive_allows_narrowing() {
        let full = CapabilityRights::ALL;
        let read_only = CapabilityRights::READ;
        assert!(full.can_derive(read_only));
        assert!(full.can_derive(CapabilityRights::NONE));
        assert!(full.can_derive(full));
    }

    #[test]
    fn can_derive_rejects_widening() {
        // The security-critical negative case: a read-only capability
        // must never be able to derive one with rights it doesn't
        // have -- this is what actually prevents privilege escalation
        // through delegation, not just an incidental property.
        let read_only = CapabilityRights::READ;
        let read_write = CapabilityRights::READ.union(CapabilityRights::WRITE);

        assert!(!read_only.can_derive(read_write));
        assert!(!read_only.can_derive(CapabilityRights::EXECUTE));
        assert!(!CapabilityRights::NONE.can_derive(CapabilityRights::READ));
    }

    #[test]
    fn is_authorized_requires_both_sufficient_trust_and_sufficient_rights() {
        assert!(is_authorized(
            SecurityLabel::Kernel,
            CapabilityRights::ALL,
            SecurityLabel::Application,
            CapabilityRights::READ,
        ));
    }

    #[test]
    fn is_authorized_denies_insufficient_rights_even_with_sufficient_trust() {
        assert!(!is_authorized(
            SecurityLabel::Kernel,
            CapabilityRights::READ,
            SecurityLabel::Application,
            CapabilityRights::WRITE,
        ));
    }

    #[test]
    fn is_authorized_denies_insufficient_trust_even_with_sufficient_rights() {
        // The security-relevant negative case: holding the right
        // capability bits isn't enough on its own if the holder's
        // trust label doesn't clear the target's -- an Application
        // holding CapabilityRights::ALL still can't act on a
        // Kernel-labeled target.
        assert!(!is_authorized(
            SecurityLabel::Application,
            CapabilityRights::ALL,
            SecurityLabel::Kernel,
            CapabilityRights::READ,
        ));
    }

    #[test]
    fn is_authorized_denies_when_both_conditions_fail() {
        assert!(!is_authorized(
            SecurityLabel::Application,
            CapabilityRights::NONE,
            SecurityLabel::Kernel,
            CapabilityRights::READ,
        ));
    }

    #[test]
    fn is_authorized_allows_equal_trust_with_matching_rights() {
        assert!(is_authorized(
            SecurityLabel::SystemService,
            CapabilityRights::READ,
            SecurityLabel::SystemService,
            CapabilityRights::READ,
        ));
    }
}
