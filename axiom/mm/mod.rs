/// Memory management scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    Kernel,
    Device,
    Reclaimable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: PhysicalAddress,
    pub length: u64,
    pub kind: MemoryRegionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapInfo {
    pub region_count: usize,
}

pub struct BootstrapAllocator;

impl BootstrapAllocator {
    pub const fn new() -> Self {
        Self
    }
}

pub fn early_mm_init() {
    // Planned:
    // 1. ingest boot memory map
    // 2. classify usable/reserved regions
    // 3. initialize bootstrap allocator
    // 4. prepare page allocator roadmap
}
