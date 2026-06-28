use core::arch::asm;

/// x86_64 port I/O abstraction.
///
/// This module centralizes architecture-specific port access and
/// intentionally contains the unsafe machine instruction boundary.
///
/// Safety design:
/// - generic kernel code must not perform raw port I/O directly
/// - consumers use Port methods only
/// - all unsafe is localized here

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

    /// Read one byte from the I/O port.
    ///
    /// Safety rationale:
    /// - executing in is inherently architecture-specific and privileged
    /// - this operation must remain isolated to this module
    pub fn read_u8(&self) -> u8 {
        let value: u8;
        unsafe {
            asm!(
                "in al, dx",
                out("al") value,
                in("dx") self.address,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    /// Write one byte to the I/O port.
    ///
    /// Safety rationale:
    /// - executing out is inherently architecture-specific and privileged
    /// - this operation must remain isolated to this module
    pub fn write_u8(&self, value: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") self.address,
                in("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
