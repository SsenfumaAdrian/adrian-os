#[cfg(not(feature = "std"))]
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

}

/// Bare-metal port access: real, privileged in/out instructions.
/// This is the only path used on the real target; it is what makes
/// this module architecture-specific and unsafe in the first place.
#[cfg(not(feature = "std"))]
impl Port {
    /// Read one byte from the I/O port.
    pub fn read_u8(&self) -> u8 {
        let value: u8;
        // SAFETY: `in al, dx` reads one byte from the I/O port named in
        // dx. Invariants relied on:
        // - CPL 0. This impl block is compiled only for the
        //   freestanding (no_std) kernel, which runs in ring 0; the
        //   hosted impl below exists precisely so userspace never
        //   reaches this instruction.
        // - No memory is read or written (`nomem`) and the stack is not
        //   touched (`nostack`); `in` does neither.
        // - `in` writes no flags, so `preserves_flags` is accurate.
        // - `value` is fully initialized by the `out("al")` operand
        //   before it is read.
        // Reading a port is not memory-safe in general -- it can have
        // device side effects -- which is why every port in this kernel
        // is reached through this one module rather than ad hoc.
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
    pub fn write_u8(&self, value: u8) {
        // SAFETY: `out dx, al` writes one byte to the I/O port named in
        // dx. Same invariants as `read_u8`: ring 0 by construction for
        // this cfg, no memory or stack access, no flags written. The
        // caller is responsible for the *device* consequences of the
        // write; this only guarantees the instruction itself is
        // well-formed and legal here.
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

/// Hosted port access: `in`/`out` are ring-0-only instructions and will
/// fault if a userspace process executes them, so hosted builds (dev
/// loop, tests) never touch real ports. Reads/writes become inert.
///
/// Visible serial output for hosted builds is handled one layer up, in
/// `debug::serial`, which knows which writes are actual message bytes
/// versus UART configuration register writes — a distinction this
/// generic port abstraction intentionally doesn't know about.
#[cfg(feature = "std")]
impl Port {
    pub fn read_u8(&self) -> u8 {
        0
    }

    pub fn write_u8(&self, _value: u8) {}
}
