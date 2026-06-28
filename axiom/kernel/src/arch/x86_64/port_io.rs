/// x86_64 port I/O abstraction scaffold.
///
/// This module centralizes architecture-specific port access concepts.
/// Actual hardware operations are intentionally placeholders for now
/// until the unsafe implementation boundary is introduced and reviewed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Port {
    address: u16,
}

impl Port {
    pub const fn new(address: u16) -> Self {
        Self { address }
    }

    pub const fn address(&self) -> u16 {
        self.address
    }

    /// Placeholder byte read.
    ///
    /// Future implementation:
    /// - architecture-specific unsafe in instruction wrapper
    pub fn read_u8(&self) -> u8 {
        let _ = self.address;
        0
    }

    /// Placeholder byte write.
    ///
    /// Future implementation:
    /// - architecture-specific unsafe out instruction wrapper
    pub fn write_u8(&self, value: u8) {
        let _ = self.address;
        let _ = value;
    }
}
