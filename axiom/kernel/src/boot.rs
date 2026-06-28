/// Boot-time structures and Halo -> Axiom handoff placeholders.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootArchitecture {
    Unknown = 0,
    X86_64 = 1,
    Arm64 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootFlags {
    pub bits: u64,
}

impl BootFlags {
    pub const NONE: Self = Self { bits: 0 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootContextHeader {
    pub magic: u64,
    pub version: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootMemoryMapInfo {
    pub entry_count: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootFramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub format: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootContext {
    pub header: BootContextHeader,
    pub architecture: BootArchitecture,
    pub flags: BootFlags,
    pub memory_map: BootMemoryMapInfo,
    pub framebuffer: BootFramebufferInfo,
}

impl BootContext {
    pub const MAGIC: u64 = 0x4144_5249_414E_4F53; // "ADRIANOS" inspired marker
    pub const VERSION: u32 = 1;

    pub const fn empty() -> Self {
        Self {
            header: BootContextHeader {
                magic: Self::MAGIC,
                version: Self::VERSION,
                reserved: 0,
            },
            architecture: BootArchitecture::Unknown,
            flags: BootFlags::NONE,
            memory_map: BootMemoryMapInfo {
                entry_count: 0,
                reserved: 0,
            },
            framebuffer: BootFramebufferInfo {
                width: 0,
                height: 0,
                pitch: 0,
                format: 0,
            },
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.header.magic == Self::MAGIC && self.header.version == Self::VERSION
    }
}

/// Placeholder boot entry used by future Halo -> Axiom integration.
pub fn boot_entry(context: &BootContext) {
    let _ = context;
}
