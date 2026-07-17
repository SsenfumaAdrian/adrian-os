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

impl MemoryRegion {
    /// One past the last byte of this region.
    pub const fn end(&self) -> PhysicalAddress {
        PhysicalAddress(self.start.0 + self.length)
    }

    pub const fn is_usable(&self) -> bool {
        matches!(self.kind, MemoryRegionKind::Usable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapInfo {
    pub region_count: usize,
}

/// Aggregate counts and byte totals across a memory map, split out by
/// region kind. Pure and hardware-independent -- built from whatever
/// `&[MemoryRegion]` the caller has, whether that's real firmware data
/// (once Halo exists) or synthetic data in a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemoryMapSummary {
    pub region_count: usize,
    pub usable_region_count: usize,
    pub usable_bytes: u64,
    pub reserved_bytes: u64,
}

/// Classify a set of memory regions into an aggregate summary.
pub fn summarize_regions(regions: &[MemoryRegion]) -> MemoryMapSummary {
    let mut summary = MemoryMapSummary {
        region_count: regions.len(),
        ..MemoryMapSummary::default()
    };

    for region in regions {
        match region.kind {
            MemoryRegionKind::Usable => {
                summary.usable_region_count += 1;
                summary.usable_bytes += region.length;
            }
            MemoryRegionKind::Reserved => {
                summary.reserved_bytes += region.length;
            }
            _ => {}
        }
    }

    summary
}

/// The largest usable region, ties broken by first-seen.
///
/// Bump-allocating out of a single region is simpler and safer to
/// reason about during early bring-up than spanning multiple regions
/// before a real physical frame allocator exists, so the bootstrap
/// allocator only ever needs one.
fn largest_usable_region(regions: &[MemoryRegion]) -> Option<&MemoryRegion> {
    regions
        .iter()
        .filter(|region| region.is_usable())
        .max_by_key(|region| region.length)
}

/// Round `value` up to the next multiple of `align` (`align` must be a
/// power of two). Returns `None` on overflow rather than panicking --
/// an allocator should fail an allocation, never crash the kernel over
/// address-space arithmetic.
fn align_up(value: u64, align: u64) -> Option<u64> {
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

/// Early bootstrap physical allocator.
///
/// Deliberately minimal: a bump allocator over a single usable region,
/// meant only to live until a real physical frame allocator exists.
/// No reclamation -- allocations are never freed individually, which
/// matches how early bring-up allocations are actually used (page
/// tables, an initial heap region: things that live for the life of
/// the kernel).
#[derive(Debug, Clone, Copy)]
pub struct BootstrapAllocator {
    cursor: PhysicalAddress,
    region_end: PhysicalAddress,
}

impl BootstrapAllocator {
    /// An allocator with nothing to allocate from yet.
    /// `allocate` returns `None` until `init` gives it a real region.
    pub const fn new() -> Self {
        Self {
            cursor: PhysicalAddress(0),
            region_end: PhysicalAddress(0),
        }
    }

    /// Seed the allocator from the largest usable region in `regions`.
    /// Returns `false` if no usable region was found.
    pub fn init(&mut self, regions: &[MemoryRegion]) -> bool {
        match largest_usable_region(regions) {
            Some(region) => {
                self.cursor = region.start;
                self.region_end = region.end();
                true
            }
            None => false,
        }
    }

    /// Bump-allocate `size` bytes aligned to `align` (must be a power
    /// of two). Returns `None` if the seeded region is exhausted, or
    /// if `size`/`align` are invalid.
    pub fn allocate(&mut self, size: u64, align: u64) -> Option<PhysicalAddress> {
        if size == 0 || align == 0 || !align.is_power_of_two() {
            return None;
        }

        let aligned = align_up(self.cursor.0, align)?;
        let end = aligned.checked_add(size)?;

        if end > self.region_end.0 {
            return None;
        }

        self.cursor = PhysicalAddress(end);
        Some(PhysicalAddress(aligned))
    }

    /// Bytes still available between the cursor and the end of the
    /// seeded region.
    pub const fn remaining(&self) -> u64 {
        self.region_end.0.saturating_sub(self.cursor.0)
    }
}

/// Main early memory-management bring-up step.
///
/// `regions` is whatever memory map the caller has. Today that's
/// always empty: there is no bootloader (Halo) yet, so there is no
/// real firmware-sourced memory map to ingest -- passing fabricated
/// regions here would look like progress without being real. The
/// classification and allocator logic above are written against the
/// real `MemoryRegion` type, so nothing here needs to change once Halo
/// starts supplying real regions; only the caller's input does.
pub fn early_mm_init(regions: &[MemoryRegion]) -> BootstrapAllocator {
    let mut allocator = BootstrapAllocator::new();
    allocator.init(regions);
    allocator
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(start: u64, length: u64, kind: MemoryRegionKind) -> MemoryRegion {
        MemoryRegion {
            start: PhysicalAddress(start),
            length,
            kind,
        }
    }

    #[test]
    fn summarize_empty_regions() {
        assert_eq!(summarize_regions(&[]), MemoryMapSummary::default());
    }

    #[test]
    fn summarize_mixed_regions() {
        let regions = [
            region(0, 0x1000, MemoryRegionKind::Reserved),
            region(0x1000, 0x9000, MemoryRegionKind::Usable),
            region(0xA000, 0x1000, MemoryRegionKind::Kernel),
        ];
        let summary = summarize_regions(&regions);
        assert_eq!(summary.region_count, 3);
        assert_eq!(summary.usable_region_count, 1);
        assert_eq!(summary.usable_bytes, 0x9000);
        assert_eq!(summary.reserved_bytes, 0x1000);
    }

    #[test]
    fn allocator_picks_largest_usable_region() {
        let regions = [
            region(0, 0x1000, MemoryRegionKind::Usable),
            region(0x2000, 0x10000, MemoryRegionKind::Usable),
            region(0x20000, 0x500, MemoryRegionKind::Reserved),
        ];
        let mut allocator = BootstrapAllocator::new();
        assert!(allocator.init(&regions));
        assert_eq!(allocator.remaining(), 0x10000);
    }

    #[test]
    fn allocator_returns_none_with_no_usable_region() {
        let regions = [region(0, 0x1000, MemoryRegionKind::Reserved)];
        let mut allocator = BootstrapAllocator::new();
        assert!(!allocator.init(&regions));
        assert_eq!(allocator.allocate(8, 8), None);
    }

    #[test]
    fn allocate_respects_alignment_and_advances_cursor() {
        let regions = [region(0x10, 0x100, MemoryRegionKind::Usable)];
        let mut allocator = BootstrapAllocator::new();
        allocator.init(&regions);

        let first = allocator.allocate(8, 16).unwrap();
        assert_eq!(first.0 % 16, 0);

        let second = allocator.allocate(8, 16).unwrap();
        assert!(second.0 > first.0);
        assert_eq!(second.0 % 16, 0);
    }

    #[test]
    fn allocate_fails_when_region_is_exhausted() {
        let regions = [region(0, 32, MemoryRegionKind::Usable)];
        let mut allocator = BootstrapAllocator::new();
        allocator.init(&regions);

        assert!(allocator.allocate(20, 1).is_some());
        assert_eq!(allocator.allocate(20, 1), None);
    }

    #[test]
    fn allocate_rejects_non_power_of_two_alignment() {
        let regions = [region(0, 0x100, MemoryRegionKind::Usable)];
        let mut allocator = BootstrapAllocator::new();
        allocator.init(&regions);
        assert_eq!(allocator.allocate(8, 3), None);
    }

    #[test]
    fn allocate_rejects_zero_size() {
        let regions = [region(0, 0x100, MemoryRegionKind::Usable)];
        let mut allocator = BootstrapAllocator::new();
        allocator.init(&regions);
        assert_eq!(allocator.allocate(0, 8), None);
    }
}
