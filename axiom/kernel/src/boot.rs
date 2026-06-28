/// Boot-time structures and kernel handoff placeholders for ADRIAN OS.

#[derive(Debug, Clone, Copy)]
pub struct BootContextHeader {
    pub magic: u64,
    pub version: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct BootContext {
    pub header: BootContextHeader,
}

impl BootContext {
    pub const fn empty() -> Self {
        Self {
            header: BootContextHeader {
                magic: 0,
                version: 1,
                reserved: 0,
            },
        }
    }
}

/// Placeholder boot entry used by future Halo -> Axiom integration.
pub fn boot_entry(context: &BootContext) {
    let _ = context;
}
