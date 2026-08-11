/// x86_64 Interrupt Descriptor Table.
///
/// This module builds and encodes IDT entries correctly -- the part
/// that's pure data layout and fully verifiable without hardware.
/// Installing real exception handlers and calling `Idt::load()` is
/// deliberately not wired up yet: a loaded IDT needs a `'static` home
/// (the CPU holds a pointer to it for as long as it's loaded), which
/// means real kernel-wide static state that doesn't exist yet.

/// Which kind of gate an IDT entry describes.
///
/// An interrupt gate clears the interrupt flag on entry (so the
/// handler itself can't be interrupted unless it explicitly re-enables
/// interrupts); a trap gate leaves it unchanged. Exceptions like
/// breakpoints are conventionally trap gates; hardware interrupt
/// handlers are conventionally interrupt gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateType {
    Interrupt,
    Trap,
}

impl GateType {
    const fn encoding(self) -> u8 {
        match self {
            GateType::Interrupt => 0xE,
            GateType::Trap => 0xF,
        }
    }
}

/// A single 16-byte long-mode IDT gate descriptor.
///
/// Layout matches the Intel SDM Vol. 3, Ch. 6 interrupt-gate format
/// exactly: the 64-bit handler offset is split across three fields
/// (low/mid/high) with the segment selector, IST index, and gate
/// attributes packed in between.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    segment_selector: u16,
    ist: u8,
    type_attributes: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    /// A not-present entry: what every slot starts as. Delivering an
    /// interrupt through a missing entry raises a fault the CPU can
    /// report, rather than jumping to a garbage address.
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            segment_selector: 0,
            ist: 0,
            type_attributes: 0, // present bit (0x80) clear
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Build a present entry pointing at `handler`.
    ///
    /// `segment_selector` is the code segment the handler runs under
    /// (normally the kernel code segment). `ist` selects an Interrupt
    /// Stack Table entry (0 = use the current stack, 1-7 = a dedicated
    /// known-good stack); only the low 3 bits are meaningful, anything
    /// else is masked off. `dpl` (0-3) is the lowest privilege level
    /// allowed to invoke this entry via `int`; masked to 2 bits.
    pub const fn new(
        handler: u64,
        segment_selector: u16,
        gate_type: GateType,
        dpl: u8,
        ist: u8,
    ) -> Self {
        let present = 0x80;
        let dpl_bits = (dpl & 0b11) << 5;
        let type_attributes = present | dpl_bits | gate_type.encoding();

        Self {
            offset_low: (handler & 0xFFFF) as u16,
            segment_selector,
            ist: ist & 0b111,
            type_attributes,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFF_FFFF) as u32,
            reserved: 0,
        }
    }

    pub const fn is_present(&self) -> bool {
        self.type_attributes & 0x80 != 0
    }

    /// Reconstructs the full 64-bit handler address from the three
    /// split offset fields.
    pub const fn handler_address(&self) -> u64 {
        (self.offset_low as u64)
            | ((self.offset_mid as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    pub const fn dpl(&self) -> u8 {
        (self.type_attributes >> 5) & 0b11
    }

    pub const fn ist(&self) -> u8 {
        self.ist
    }
}

/// The full 256-entry table.
#[repr(C, packed)]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); 256],
        }
    }

    /// Install a handler at interrupt vector `vector`.
    pub fn set_handler(
        &mut self,
        vector: u8,
        handler: u64,
        segment_selector: u16,
        gate_type: GateType,
    ) {
        self.entries[vector as usize] = IdtEntry::new(handler, segment_selector, gate_type, 0, 0);
    }

    pub fn entry(&self, vector: u8) -> IdtEntry {
        self.entries[vector as usize]
    }
}

/// The pseudo-descriptor `lidt` reads: table size minus one, then the
/// table's linear address. Only meaningful on the real `lidt` path --
/// gated the same way so it doesn't sit unused under `--features std`.
#[cfg(not(feature = "std"))]
#[repr(C, packed)]
struct IdtDescriptor {
    limit: u16,
    base: u64,
}

impl Idt {
    /// Load this table into the CPU's IDTR.
    ///
    /// `lidt` is privileged -- ring 3 (where an ordinary hosted
    /// process runs) faults on it. `&'static self` is required because
    /// the CPU keeps referencing this memory for as long as it's
    /// loaded; a stack-local table would leave a dangling reference
    /// the moment the caller returned.
    #[cfg(not(feature = "std"))]
    pub unsafe fn load(&'static self) {
        let descriptor = IdtDescriptor {
            limit: (core::mem::size_of::<Idt>() - 1) as u16,
            base: self as *const Idt as u64,
        };
        core::arch::asm!(
            "lidt [{0}]",
            in(reg) &descriptor,
            options(readonly, nostack, preserves_flags)
        );
    }

    /// Hosted stand-in: there is no ring 0 to load a real IDT into
    /// when running as an ordinary process, so this deliberately does
    /// nothing. Mirrors the split already established in `port_io.rs`.
    #[cfg(feature = "std")]
    pub unsafe fn load(&'static self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_size_is_sixteen_bytes() {
        assert_eq!(core::mem::size_of::<IdtEntry>(), 16);
    }

    #[test]
    fn idt_size_is_4096_bytes() {
        assert_eq!(core::mem::size_of::<Idt>(), 256 * 16);
    }

    #[test]
    fn missing_entry_is_not_present() {
        assert!(!IdtEntry::missing().is_present());
    }

    #[test]
    fn new_entry_is_present() {
        let entry = IdtEntry::new(0x1000, 0x08, GateType::Interrupt, 0, 0);
        assert!(entry.is_present());
    }

    #[test]
    fn handler_address_round_trips_across_split_offset_fields() {
        // Exercises every split field (low/mid/high) with values that
        // have bits set across all three, so a masking or shift
        // mistake in any one field would show up here.
        let addresses = [
            0x0000_0000_0000_0000u64,
            0x0000_0000_0000_FFFF,
            0x0000_0000_FFFF_0000,
            0xFFFF_FFFF_0000_0000,
            0xDEAD_BEEF_CAFE_F00D,
            0xFFFF_FFFF_FFFF_FFFF,
        ];

        for &addr in &addresses {
            let entry = IdtEntry::new(addr, 0x08, GateType::Interrupt, 0, 0);
            assert_eq!(
                entry.handler_address(),
                addr,
                "address 0x{:X} did not round-trip",
                addr
            );
        }
    }

    #[test]
    fn dpl_round_trips() {
        for dpl in 0..=3u8 {
            let entry = IdtEntry::new(0, 0, GateType::Trap, dpl, 0);
            assert_eq!(entry.dpl(), dpl);
        }
    }

    #[test]
    fn dpl_is_masked_to_two_bits() {
        // 0b101 (5) is out of range for a 2-bit field; only the low
        // two bits should survive encoding.
        let entry = IdtEntry::new(0, 0, GateType::Trap, 0b101, 0);
        assert_eq!(entry.dpl(), 0b01);
    }

    #[test]
    fn ist_is_masked_to_three_bits() {
        let entry = IdtEntry::new(0, 0, GateType::Interrupt, 0, 0xFF);
        assert_eq!(entry.ist(), 0b111);
    }

    #[test]
    fn gate_type_is_encoded_distinctly() {
        let interrupt = IdtEntry::new(0, 0, GateType::Interrupt, 0, 0);
        let trap = IdtEntry::new(0, 0, GateType::Trap, 0, 0);
        assert_ne!(interrupt.type_attributes, trap.type_attributes);
    }

    #[test]
    fn new_idt_has_all_entries_missing() {
        let idt = Idt::new();
        for vector in 0..=255u8 {
            assert!(
                !idt.entry(vector).is_present(),
                "vector {} should start missing",
                vector
            );
        }
    }

    #[test]
    fn set_handler_populates_only_the_targeted_vector() {
        let mut idt = Idt::new();
        idt.set_handler(14, 0xABCD_0000, 0x08, GateType::Interrupt); // 14 = page fault

        assert!(idt.entry(14).is_present());
        assert_eq!(idt.entry(14).handler_address(), 0xABCD_0000);

        // Neighbors should be untouched.
        assert!(!idt.entry(13).is_present());
        assert!(!idt.entry(15).is_present());
    }
}
