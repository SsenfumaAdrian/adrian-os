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

pub fn early_security_init() {
    // CapabilityRights and SecurityLabel now have real logic --
    // rights composition (union/intersection/without) and the
    // can_derive invariant, plus a trust ordering on labels -- proven
    // correct by this module's own tests. No syscall policy
    // integration or object access control hooks yet: those need
    // syscall dispatch (syscall.rs) to actually call into this, which
    // isn't wired up on that side either yet.
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
}
