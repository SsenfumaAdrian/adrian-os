use crate::arch::x86_64::port_io::Port;

/// 8259 Programmable Interrupt Controller remapping.
///
/// On boot, the PIC maps IRQ0-7 to interrupt vectors 0x08-0x0F --
/// which collides directly with CPU exception vectors (0x08 is the
/// double-fault exception, for instance). Remapping moves IRQs
/// somewhere clear of that range before interrupts are ever enabled.

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const ICW1_INIT_EXPECT_ICW4: u8 = 0x11;
const ICW3_MASTER_SLAVE_AT_IRQ2: u8 = 0x04;
const ICW3_SLAVE_CASCADE_IDENTITY: u8 = 0x02;
const ICW4_8086_MODE: u8 = 0x01;

/// One (port, value) write in the remap sequence. Pure data -- the
/// actual port I/O executing it is gated the same way as everything
/// else touching hardware; this sequence itself is what's tested here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PicWrite {
    pub port: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PicRemap {
    pub master_offset: u8,
    pub slave_offset: u8,
}

impl PicRemap {
    /// IRQ0-7 -> 0x20-0x27, IRQ8-15 -> 0x28-0x2F: clear of the CPU
    /// exception range (0x00-0x1F) with no gap or overlap between the
    /// two PICs' eight lines each.
    pub const fn standard() -> Self {
        Self {
            master_offset: 0x20,
            slave_offset: 0x28,
        }
    }

    /// The exact writes, in order, that perform the remap: ICW1 to
    /// both PICs' command ports, then ICW2-4 to both data ports.
    pub const fn sequence(&self) -> [PicWrite; 8] {
        [
            PicWrite { port: PIC1_COMMAND, value: ICW1_INIT_EXPECT_ICW4 },
            PicWrite { port: PIC2_COMMAND, value: ICW1_INIT_EXPECT_ICW4 },
            PicWrite { port: PIC1_DATA, value: self.master_offset },
            PicWrite { port: PIC2_DATA, value: self.slave_offset },
            PicWrite { port: PIC1_DATA, value: ICW3_MASTER_SLAVE_AT_IRQ2 },
            PicWrite { port: PIC2_DATA, value: ICW3_SLAVE_CASCADE_IDENTITY },
            PicWrite { port: PIC1_DATA, value: ICW4_8086_MODE },
            PicWrite { port: PIC2_DATA, value: ICW4_8086_MODE },
        ]
    }

    /// Execute the remap sequence against the real PIC ports.
    ///
    /// Safe on the bare-metal path in the same sense the rest of
    /// `Port` is: it's real, privileged I/O, but the unsafety is
    /// already fully encapsulated inside `Port::write_u8`. Under
    /// `--features std`, `Port::write_u8` is already a no-op, so this
    /// needs no separate gating of its own -- it inherits the split.
    pub fn apply(&self) {
        for write in self.sequence() {
            Port::new(write.port).write_u8(write.value);
        }
    }
}

/// Mask (disable) every IRQ line on both PICs. After remap, the same
/// data ports become each PIC's Interrupt Mask Register; writing all
/// 1-bits masks every line. Defense in depth alongside the CPU's own
/// interrupt flag (never enabled anywhere in this codebase yet) --
/// nothing should be able to fire before a handler exists for it.
pub fn mask_all() {
    Port::new(PIC1_DATA).write_u8(0xFF);
    Port::new(PIC2_DATA).write_u8(0xFF);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_offsets_are_clear_of_cpu_exceptions() {
        let remap = PicRemap::standard();
        // CPU exceptions occupy vectors 0x00-0x1F (0-31).
        assert!(remap.master_offset >= 32);
        assert!(remap.slave_offset >= 32);
    }

    #[test]
    fn standard_offsets_are_eight_apart() {
        // Each PIC handles 8 IRQ lines; the slave's block must start
        // exactly where the master's ends, with no gap or overlap.
        let remap = PicRemap::standard();
        assert_eq!(remap.slave_offset - remap.master_offset, 8);
    }

    #[test]
    fn sequence_writes_icw1_to_both_command_ports_first() {
        let sequence = PicRemap::standard().sequence();
        assert_eq!(sequence[0], PicWrite { port: PIC1_COMMAND, value: ICW1_INIT_EXPECT_ICW4 });
        assert_eq!(sequence[1], PicWrite { port: PIC2_COMMAND, value: ICW1_INIT_EXPECT_ICW4 });
    }

    #[test]
    fn sequence_writes_vector_offsets_to_data_ports() {
        let remap = PicRemap { master_offset: 0x40, slave_offset: 0x48 };
        let sequence = remap.sequence();
        assert_eq!(sequence[2], PicWrite { port: PIC1_DATA, value: 0x40 });
        assert_eq!(sequence[3], PicWrite { port: PIC2_DATA, value: 0x48 });
    }

    #[test]
    fn sequence_never_touches_a_port_outside_the_pic_range() {
        for write in PicRemap::standard().sequence() {
            assert!(
                write.port == PIC1_COMMAND
                    || write.port == PIC1_DATA
                    || write.port == PIC2_COMMAND
                    || write.port == PIC2_DATA,
                "unexpected port 0x{:X}",
                write.port
            );
        }
    }

    #[test]
    fn custom_offsets_propagate_into_the_sequence() {
        let remap = PicRemap { master_offset: 0x60, slave_offset: 0x68 };
        let sequence = remap.sequence();
        assert!(sequence.contains(&PicWrite { port: PIC1_DATA, value: 0x60 }));
        assert!(sequence.contains(&PicWrite { port: PIC2_DATA, value: 0x68 }));
    }

    #[test]
    fn apply_does_not_panic_on_the_hosted_path() {
        // Port::write_u8 is a no-op under --features std; this just
        // confirms the call chain itself is sound.
        PicRemap::standard().apply();
    }

    #[test]
    fn mask_all_does_not_panic_on_the_hosted_path() {
        mask_all();
    }
}
