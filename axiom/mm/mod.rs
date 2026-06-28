/// Memory management scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    Kernel,
    Device,
    Unknown,
}

pub fn early_mm_init() {
    // Planned:
    // - boot memory map ingest
    // - bootstrap allocator
    // - early page allocator
    // - virtual memory initialization
}
