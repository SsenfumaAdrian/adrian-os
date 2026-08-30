/// x86_64 4-level paging (PML4 -> PDPT -> PD -> PT).
///
/// Entry encoding and address splitting are pure data manipulation --
/// fully verifiable without hardware, same as idt.rs's entry encoding.
/// Actually pointing CR3 at a table and flipping on paging is real
/// privileged state with no way to test it outside real or
/// virtualized hardware, so that step isn't attempted here.
///
/// `SoftwarePageMapper` below *does* walk and build tables, which this
/// header previously said was deliberately not attempted. The reason
/// given was real: whether a physical address is dereferenceable while
/// building tables depends on the boot-time memory model, which depends
/// on firmware/Halo integration that still doesn't exist. That question
/// is not settled -- it is now an explicit *parameter* instead. The
/// mapper takes a `phys_offset` and requires the caller to guarantee a
/// live direct map of physical memory at that offset. With
/// `phys_offset = 0` it means identity-mapped.
///
/// So the assumption did not go away; it moved into the constructor
/// where it can be stated and audited. Until Halo can supply a real
/// offset, the mapper has no production callers -- only tests, which
/// supply their own backing memory. Do not read its existence as
/// evidence that the boot memory model is decided.
use crate::mm::{PhysicalAddress, VirtualAddress};

const PRESENT_BIT: u64 = 1 << 0;
const WRITABLE_BIT: u64 = 1 << 1;
const USER_BIT: u64 = 1 << 2;
/// Bit 7, "PS" (page size). On a PDPT or PD entry it means the entry is
/// a *leaf* mapping a 1 GiB or 2 MiB page, not a pointer to the next
/// table. A walker that ignores it will treat data as a page table:
/// reads return nonsense, and writes corrupt whatever the huge page
/// maps. Firmware and most bootloaders use 2 MiB pages for the early
/// map, so this is the common case, not an exotic one.
const HUGE_PAGE_BIT: u64 = 1 << 7;
/// Bits 12-51: where the physical frame address lives in an entry.
/// Bits 0-11 are flags; bits 52-63 are reserved/other (NX etc., not
/// modeled yet).
const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// The flags that matter for basic mapping. PWT, PCD, accessed,
/// dirty, huge-page, global, and NX all exist on real hardware but
/// aren't modeled yet -- added when something actually needs to set
/// them, not spun up speculatively now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags {
    pub present: bool,
    pub writable: bool,
    pub user_accessible: bool,
}

/// A single page table entry: physical frame address plus flags,
/// packed into 8 bytes exactly as the hardware expects.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// A not-present entry: what every slot starts as.
    pub const fn missing() -> Self {
        Self(0)
    }

    pub const fn new(physical_address: PhysicalAddress, flags: PageFlags) -> Self {
        let mut bits = physical_address.0 & ADDRESS_MASK;
        if flags.present {
            bits |= PRESENT_BIT;
        }
        if flags.writable {
            bits |= WRITABLE_BIT;
        }
        if flags.user_accessible {
            bits |= USER_BIT;
        }
        Self(bits)
    }

    pub const fn is_present(&self) -> bool {
        self.0 & PRESENT_BIT != 0
    }

    pub const fn is_writable(&self) -> bool {
        self.0 & WRITABLE_BIT != 0
    }

    pub const fn is_user_accessible(&self) -> bool {
        self.0 & USER_BIT != 0
    }

    /// Whether bit 7 (PS) is set. Only meaningful on a PDPT or PD entry,
    /// where it marks a 1 GiB or 2 MiB leaf. `PageFlags` cannot set this
    /// bit, so entries built by `new` never report true -- but entries
    /// *read back* from a table firmware or a bootloader built very
    /// often do, which is the whole reason this accessor exists.
    pub const fn is_huge_page(&self) -> bool {
        self.0 & HUGE_PAGE_BIT != 0
    }

    /// The same entry with PS set.
    ///
    /// This does **not** make huge-page mapping a supported operation --
    /// nothing here computes 2 MiB or 1 GiB alignment or reserved-bit
    /// requirements. It exists so the huge-page paths in the walker can be
    /// exercised by tests, since a firmware-built table cannot be
    /// conjured up otherwise and those paths would go untested.
    #[must_use]
    pub const fn marked_huge(self) -> Self {
        Self(self.0 | HUGE_PAGE_BIT)
    }

    /// Whether this entry can be followed to a next-level table: present
    /// and not a huge-page leaf.
    pub const fn points_to_table(&self) -> bool {
        self.is_present() && !self.is_huge_page()
    }

    /// The physical frame address, with the low 12 flag bits masked
    /// away regardless of what was passed to `new` -- a page frame is
    /// always 4 KiB aligned, so those bits are never really address.
    pub const fn physical_address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & ADDRESS_MASK)
    }
}

pub const ENTRIES_PER_TABLE: usize = 512;

/// One level of the page table hierarchy. `align(4096)`: a page
/// table's own address, wherever it ends up, must itself be page-
/// aligned -- entries only store the upper address bits, implicitly
/// assuming the lower 12 are zero. 512 entries x 8 bytes is exactly
/// one 4 KiB page, tested below as a real structural invariant, not
/// just a comment.
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::missing(); ENTRIES_PER_TABLE],
        }
    }

    pub fn entry(&self, index: usize) -> PageTableEntry {
        self.entries[index]
    }

    pub fn set_entry(&mut self, index: usize, entry: PageTableEntry) {
        self.entries[index] = entry;
    }
}

/// The four table-index components plus final byte offset a virtual
/// address decomposes into: 9 bits each for PML4/PDPT/PD/PT (2^9 =
/// 512, matching ENTRIES_PER_TABLE), 12 bits of offset within the
/// final page (2^12 = 4096, matching the standard 4 KiB page size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualAddressParts {
    pub pml4_index: usize,
    pub pdpt_index: usize,
    pub pd_index: usize,
    pub pt_index: usize,
    pub offset: usize,
}

impl VirtualAddress {
    /// Whether this is a canonical 48-bit address: bits 48-63 must all
    /// equal bit 47. Hardware raises #GP on a non-canonical address, but
    /// `split` silently discards bits 48-63, so without this check
    /// `0x0000_8000_0000_0000` and `0xFFFF_8000_0000_0000` decompose to
    /// the same four indices -- a non-canonical address would be mapped
    /// as if it were the higher-half one it aliases.
    pub const fn is_canonical(&self) -> bool {
        // Sign-extend bit 47 across the top 16 bits and require that the
        // result is unchanged.
        ((self.0 << 16) as i64 >> 16) as u64 == self.0
    }

    pub const fn split(&self) -> VirtualAddressParts {
        VirtualAddressParts {
            pml4_index: ((self.0 >> 39) & 0x1FF) as usize,
            pdpt_index: ((self.0 >> 30) & 0x1FF) as usize,
            pd_index: ((self.0 >> 21) & 0x1FF) as usize,
            pt_index: ((self.0 >> 12) & 0x1FF) as usize,
            offset: (self.0 & 0xFFF) as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMappingError {
    AlreadyMapped,
    NotMapped,
    AllocationFailed,
    InvalidAddress,
    /// The walk hit a present PDPT or PD entry with the PS bit set: a
    /// 1 GiB or 2 MiB leaf, not a table pointer. Splitting a huge page
    /// into 4 KiB pages is a real operation, but it needs a frame
    /// allocator commitment and a TLB shootdown, so it is refused here
    /// rather than half-done.
    HugePagePresent,
}

/// Software-driven 4-level page table mapper (PML4 -> PDPT -> PD -> PT).
/// Operates on a root PML4 table, walking and creating intermediate
/// tables as needed.
///
/// Every method that follows an entry dereferences a physical address as
/// `phys + phys_offset`. That is only valid under the contract stated on
/// `new`. Nothing in this type can check it.
pub struct SoftwarePageMapper<'a> {
    pml4: &'a mut PageTable,
    /// Virtual offset at which all of physical memory is mapped, so that
    /// physical address `p` is readable at `p + phys_offset`.
    phys_offset: u64,
}

impl<'a> SoftwarePageMapper<'a> {
    /// # Correctness contract
    ///
    /// The caller guarantees that all of physical memory is mapped and
    /// live at `phys_offset` -- i.e. that for every physical address `p`
    /// this mapper may touch, `p + phys_offset` is a valid, writable
    /// virtual address. `phys_offset = 0` asserts an identity map.
    ///
    /// This is not checkable here and is not yet established anywhere in
    /// the boot path; see the module header.
    pub fn new(pml4: &'a mut PageTable, phys_offset: u64) -> Self {
        Self { pml4, phys_offset }
    }

    /// Physical address of a page table, as a pointer into the direct map.
    ///
    /// `wrapping_add` rather than `+`: a higher-half `phys_offset` such
    /// as `0xFFFF_8000_0000_0000` overflows `u64` for large `phys` and
    /// would panic in debug builds, in the middle of a page-table walk.
    /// Wrapping is the intended arithmetic for a direct-map offset.
    fn table_ptr(&self, phys: PhysicalAddress) -> *mut PageTable {
        let virt = phys.0.wrapping_add(self.phys_offset);
        // A page table's address must be 4 KiB aligned: `PageTable` is
        // `align(4096)`, and building a reference to a misaligned value
        // of it is undefined behaviour outright, not merely slow.
        debug_assert!(
            virt % 4096 == 0,
            "page table physical address is not 4 KiB aligned"
        );
        virt as *mut PageTable
    }

    /// # Safety
    /// The direct-map contract on `new` must hold, and `phys` must be the
    /// address of a live `PageTable`. Borrows `self` mutably for as long
    /// as the returned reference lives, so two of these cannot coexist --
    /// deliberately, since aliasing `&mut` to the same table (reachable
    /// through a self-mapped or recursive entry) would be UB.
    unsafe fn phys_to_table_mut(&mut self, phys: PhysicalAddress) -> &mut PageTable {
        &mut *self.table_ptr(phys)
    }

    /// # Safety
    /// As `phys_to_table_mut`, for shared access.
    unsafe fn phys_to_table(&self, phys: PhysicalAddress) -> &PageTable {
        &*self.table_ptr(phys)
    }

    /// Walk the 4-level hierarchy and translate a `VirtualAddress` to its
    /// mapped `PhysicalAddress`.
    ///
    /// Resolves 1 GiB and 2 MiB leaves as well as 4 KiB ones. It would be
    /// easier to refuse huge pages here, but translation is the operation
    /// most likely to be pointed at a firmware-built table, and firmware
    /// maps with huge pages -- refusing would report "unmapped" for memory
    /// that is very much mapped.
    pub fn translate(&self, virt: VirtualAddress) -> Option<PhysicalAddress> {
        if !virt.is_canonical() {
            return None;
        }
        let parts = virt.split();

        let pml4e = self.pml4.entry(parts.pml4_index);
        if !pml4e.is_present() {
            return None;
        }
        // PS is reserved (must be zero) in a PML4 entry, so there is no
        // 512 GiB leaf to check for here.

        let pdpt = unsafe { self.phys_to_table(pml4e.physical_address()) };
        let pdpte = pdpt.entry(parts.pdpt_index);
        if !pdpte.is_present() {
            return None;
        }
        if pdpte.is_huge_page() {
            // 1 GiB leaf: base is bits 30-51, offset is the low 30 bits.
            let base = pdpte.physical_address().0 & !0x3FFF_FFFF;
            return Some(PhysicalAddress(base + (virt.0 & 0x3FFF_FFFF)));
        }

        let pd = unsafe { self.phys_to_table(pdpte.physical_address()) };
        let pde = pd.entry(parts.pd_index);
        if !pde.is_present() {
            return None;
        }
        if pde.is_huge_page() {
            // 2 MiB leaf: base is bits 21-51, offset is the low 21 bits.
            let base = pde.physical_address().0 & !0x1F_FFFF;
            return Some(PhysicalAddress(base + (virt.0 & 0x1F_FFFF)));
        }

        let pt = unsafe { self.phys_to_table(pde.physical_address()) };
        let pte = pt.entry(parts.pt_index);
        if !pte.is_present() {
            return None;
        }

        Some(PhysicalAddress(pte.physical_address().0 + parts.offset as u64))
    }

    /// Map a virtual page to a physical frame, allocating intermediate
    /// page tables via `allocator` as needed.
    ///
    /// Two limits worth knowing before relying on this:
    ///
    /// - **No rollback.** If allocation succeeds at PML4 and PDPT level
    ///   but fails at PD level, the tables already linked stay linked.
    ///   They are harmless (empty, not-present entries) but they are also
    ///   unreclaimable, because `BootstrapAllocator` is a bump allocator
    ///   with no free. Undoing the links without being able to return the
    ///   frames would trade a leaked frame for a leaked frame plus more
    ///   code, so it is not attempted.
    /// - **No TLB invalidation.** Nothing here executes `invlpg` or
    ///   reloads CR3, because CR3 never points at these tables yet. Once
    ///   it does, every `map_page`/`unmap_page` on a live hierarchy needs
    ///   invalidation or the CPU will keep using stale translations.
    pub fn map_page(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        flags: PageFlags,
        allocator: &mut crate::mm::BootstrapAllocator,
    ) -> Result<(), PageMappingError> {
        if !virt.is_canonical() {
            return Err(PageMappingError::InvalidAddress);
        }
        let parts = virt.split();
        // Copied out before `self` is mutably reborrowed below, since the
        // walk holds a `&mut` into a table for the length of each step.
        let phys_offset = self.phys_offset;

        // 1. PML4 -> PDPT
        let pdpt_phys =
            Self::get_or_create_table_at(self.pml4, parts.pml4_index, phys_offset, flags, allocator)?;
        let pdpt = unsafe { self.phys_to_table_mut(pdpt_phys) };

        // 2. PDPT -> PD
        let pd_phys =
            Self::get_or_create_table_at(pdpt, parts.pdpt_index, phys_offset, flags, allocator)?;
        let pd = unsafe { self.phys_to_table_mut(pd_phys) };

        // 3. PD -> PT
        let pt_phys =
            Self::get_or_create_table_at(pd, parts.pd_index, phys_offset, flags, allocator)?;
        let pt = unsafe { self.phys_to_table_mut(pt_phys) };

        // 4. PT entry setting
        let pte = pt.entry(parts.pt_index);
        if pte.is_present() {
            return Err(PageMappingError::AlreadyMapped);
        }

        pt.set_entry(parts.pt_index, PageTableEntry::new(phys, flags));
        Ok(())
    }

    /// Unmap a virtual page, returning its physical frame address.
    ///
    /// Does not free the frame (the caller owns it), does not reclaim
    /// intermediate tables that are now empty, and does not invalidate the
    /// TLB -- see `map_page` for why the last one is not yet needed.
    pub fn unmap_page(&mut self, virt: VirtualAddress) -> Result<PhysicalAddress, PageMappingError> {
        if !virt.is_canonical() {
            return Err(PageMappingError::InvalidAddress);
        }
        let parts = virt.split();

        let pml4e = self.pml4.entry(parts.pml4_index);
        if !pml4e.is_present() {
            return Err(PageMappingError::NotMapped);
        }

        let pdpt = unsafe { self.phys_to_table_mut(pml4e.physical_address()) };
        let pdpte = pdpt.entry(parts.pdpt_index);
        if !pdpte.is_present() {
            return Err(PageMappingError::NotMapped);
        }
        if pdpte.is_huge_page() {
            return Err(PageMappingError::HugePagePresent);
        }

        let pd = unsafe { self.phys_to_table_mut(pdpte.physical_address()) };
        let pde = pd.entry(parts.pd_index);
        if !pde.is_present() {
            return Err(PageMappingError::NotMapped);
        }
        if pde.is_huge_page() {
            return Err(PageMappingError::HugePagePresent);
        }

        let pt = unsafe { self.phys_to_table_mut(pde.physical_address()) };
        let pte = pt.entry(parts.pt_index);
        if !pte.is_present() {
            return Err(PageMappingError::NotMapped);
        }

        let phys_frame = pte.physical_address();
        pt.set_entry(parts.pt_index, PageTableEntry::missing());
        Ok(phys_frame)
    }

    /// Return the next-level table under `parent[index]`, creating it if
    /// absent.
    ///
    /// `leaf_flags` are the flags requested for the *final* 4 KiB page.
    /// Intermediate entries are not given a blanket `user_accessible:
    /// true` -- on x86_64 a page is user-accessible only if U/S is set at
    /// every level, so a permissive intermediate is how a kernel page ends
    /// up reachable from ring 3. Instead U/S is set only when the leaf
    /// wants it, and an existing intermediate is *widened* if a later user
    /// mapping needs access through a table first created for a kernel
    /// one. W is set unconditionally, since a read-only intermediate would
    /// block writes to every writable leaf beneath it; per-page write
    /// permission comes from the leaf entry.
    fn get_or_create_table_at(
        parent: &mut PageTable,
        index: usize,
        phys_offset: u64,
        leaf_flags: PageFlags,
        allocator: &mut crate::mm::BootstrapAllocator,
    ) -> Result<PhysicalAddress, PageMappingError> {
        let entry = parent.entry(index);
        if entry.is_present() {
            if entry.is_huge_page() {
                // Present, but a 1 GiB/2 MiB leaf rather than a table
                // pointer. Following it would treat mapped data as a page
                // table and write an entry into it.
                return Err(PageMappingError::HugePagePresent);
            }
            let table_phys = entry.physical_address();
            if leaf_flags.user_accessible && !entry.is_user_accessible() {
                parent.set_entry(
                    index,
                    PageTableEntry::new(
                        table_phys,
                        PageFlags {
                            present: true,
                            writable: true,
                            user_accessible: true,
                        },
                    ),
                );
            }
            Ok(table_phys)
        } else {
            let new_table_phys = allocator
                .allocate(4096, 4096)
                .ok_or(PageMappingError::AllocationFailed)?;

            let virt = new_table_phys.0.wrapping_add(phys_offset);
            debug_assert!(
                virt % 4096 == 0,
                "freshly allocated page table is not 4 KiB aligned"
            );
            let new_table = unsafe { &mut *(virt as *mut PageTable) };
            *new_table = PageTable::new();

            parent.set_entry(
                index,
                PageTableEntry::new(
                    new_table_phys,
                    PageFlags {
                        present: true,
                        writable: true,
                        user_accessible: leaf_flags.user_accessible,
                    },
                ),
            );
            Ok(new_table_phys)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_is_eight_bytes() {
        assert_eq!(core::mem::size_of::<PageTableEntry>(), 8);
    }

    #[test]
    fn table_is_exactly_one_page_and_page_aligned() {
        assert_eq!(core::mem::size_of::<PageTable>(), 4096);
        assert_eq!(core::mem::align_of::<PageTable>(), 4096);
    }

    #[test]
    fn missing_entry_is_not_present() {
        assert!(!PageTableEntry::missing().is_present());
    }

    #[test]
    fn flags_round_trip_independently() {
        let addr = PhysicalAddress(0x1000);

        let readonly = PageTableEntry::new(
            addr,
            PageFlags { present: true, writable: false, user_accessible: false },
        );
        assert!(readonly.is_present());
        assert!(!readonly.is_writable());
        assert!(!readonly.is_user_accessible());

        let user_writable = PageTableEntry::new(
            addr,
            PageFlags { present: true, writable: true, user_accessible: true },
        );
        assert!(user_writable.is_present());
        assert!(user_writable.is_writable());
        assert!(user_writable.is_user_accessible());
    }

    #[test]
    fn physical_address_masks_out_non_address_bits() {
        // A physical address with garbage in the low 12 bits (real
        // hardware would never hand back a non-page-aligned frame,
        // but this confirms encoding doesn't leak flag-region bits
        // into the returned address).
        let garbage_low_bits = PhysicalAddress(0x1000 | 0xFFF);
        let entry = PageTableEntry::new(
            garbage_low_bits,
            PageFlags { present: true, writable: true, user_accessible: false },
        );
        assert_eq!(entry.physical_address().0, 0x1000);
        assert!(entry.is_present());
        assert!(entry.is_writable());
    }

    #[test]
    fn new_table_has_all_entries_missing() {
        let table = PageTable::new();
        for i in 0..ENTRIES_PER_TABLE {
            assert!(!table.entry(i).is_present(), "index {} should start missing", i);
        }
    }

    #[test]
    fn set_entry_populates_only_the_targeted_index() {
        let mut table = PageTable::new();
        table.set_entry(
            5,
            PageTableEntry::new(
                PhysicalAddress(0x2000),
                PageFlags { present: true, writable: true, user_accessible: false },
            ),
        );
        assert!(table.entry(5).is_present());
        assert!(!table.entry(4).is_present());
        assert!(!table.entry(6).is_present());
    }

    #[test]
    fn split_round_trips_each_index_independently() {
        // Build a virtual address from known, distinct index values
        // using the inverse of split()'s own formula, then confirm
        // split() recovers exactly those values -- exercises every
        // field the same way idt.rs's handler-address test does.
        let pml4 = 0x1ABusize;
        let pdpt = 0x0CDusize;
        let pd = 0x1EFusize;
        let pt = 0x055usize;
        let offset = 0xABCusize;

        let address = VirtualAddress(
            ((pml4 as u64) << 39)
                | ((pdpt as u64) << 30)
                | ((pd as u64) << 21)
                | ((pt as u64) << 12)
                | (offset as u64),
        );

        let parts = address.split();
        assert_eq!(parts.pml4_index, pml4);
        assert_eq!(parts.pdpt_index, pdpt);
        assert_eq!(parts.pd_index, pd);
        assert_eq!(parts.pt_index, pt);
        assert_eq!(parts.offset, offset);
    }

    #[test]
    fn split_of_zero_address_is_all_zero() {
        let parts = VirtualAddress(0).split();
        assert_eq!(parts.pml4_index, 0);
        assert_eq!(parts.pdpt_index, 0);
        assert_eq!(parts.pd_index, 0);
        assert_eq!(parts.pt_index, 0);
        assert_eq!(parts.offset, 0);
    }

    #[test]
    fn software_mapper_maps_translates_and_unmaps_page() {
        use crate::mm::{MemoryRegion, MemoryRegionKind};

        let mut pml4 = PageTable::new();
        // Allocate a memory backing buffer for simulated physical memory allocation
        let mut mem_buffer = [0u8; 4096 * 10];
        let buf_ptr = mem_buffer.as_mut_ptr() as u64;

        let regions = [MemoryRegion {
            start: PhysicalAddress(buf_ptr),
            length: 4096 * 10,
            kind: MemoryRegionKind::Usable,
        }];
        let mut allocator = crate::mm::BootstrapAllocator::new();
        assert!(allocator.init(&regions));

        let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);

        let virt = VirtualAddress(0x0000_7FFF_FFFF_0000);
        let phys = PhysicalAddress(0x1000_0000);
        let flags = PageFlags {
            present: true,
            writable: true,
            user_accessible: true,
        };

        // Before mapping, translation returns None
        assert_eq!(mapper.translate(virt), None);

        // Perform mapping
        assert_eq!(mapper.map_page(virt, phys, flags, &mut allocator), Ok(()));

        // Translation returns the mapped physical frame
        assert_eq!(mapper.translate(virt), Some(phys));

        // Mapping same virtual address again returns AlreadyMapped
        assert_eq!(
            mapper.map_page(virt, phys, flags, &mut allocator),
            Err(PageMappingError::AlreadyMapped)
        );

        // Unmapping returns the original physical address
        assert_eq!(mapper.unmap_page(virt), Ok(phys));

        // After unmapping, translation returns None
        assert_eq!(mapper.translate(virt), None);

        // Unmapping again returns NotMapped
        assert_eq!(mapper.unmap_page(virt), Err(PageMappingError::NotMapped));
    }

    /// Backing store for the mapper tests: a 4 KiB-aligned buffer the test
    /// owns, plus a bump allocator seeded over it.
    ///
    /// The buffer is deliberately **not** returned from the helper. The
    /// allocator stores raw physical addresses taken from
    /// `bytes.as_mut_ptr()`, so moving the buffer out of a constructor
    /// would leave the allocator pointing at the old stack slot and every
    /// table write would land somewhere unrelated. The test owns the
    /// buffer; the helper only borrows it.
    #[repr(C, align(4096))]
    struct FakePhysicalMemory {
        bytes: [u8; 4096 * 12],
    }

    impl FakePhysicalMemory {
        fn new() -> Self {
            Self { bytes: [0u8; 4096 * 12] }
        }
    }

    fn allocator_over(memory: &mut FakePhysicalMemory) -> crate::mm::BootstrapAllocator {
        use crate::mm::{MemoryRegion, MemoryRegionKind};

        let regions = [MemoryRegion {
            start: PhysicalAddress(memory.bytes.as_mut_ptr() as u64),
            length: 4096 * 12,
            kind: MemoryRegionKind::Usable,
        }];
        let mut allocator = crate::mm::BootstrapAllocator::new();
        assert!(allocator.init(&regions));
        allocator
    }

    #[test]
    fn translate_adds_the_offset_within_the_page() {
        let mut memory = FakePhysicalMemory::new();
        let mut allocator = allocator_over(&mut memory);
        let mut pml4 = PageTable::new();
        let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);

        let page = VirtualAddress(0x0000_7FFF_FFFF_0000);
        let frame = PhysicalAddress(0x1000_0000);
        let flags = PageFlags { present: true, writable: true, user_accessible: false };
        assert_eq!(mapper.map_page(page, frame, flags, &mut allocator), Ok(()));

        // The interesting case the original test missed: a virtual address
        // partway into the page must land the same distance into the frame.
        // With a zero offset the `+ parts.offset` in `translate` could be
        // deleted and the test would still pass.
        let inside = VirtualAddress(page.0 + 0xABC);
        assert_eq!(mapper.translate(inside), Some(PhysicalAddress(frame.0 + 0xABC)));

        // A different page in the same page table stays unmapped -- so the
        // walk is really reaching the leaf index, not just the table.
        let neighbour = VirtualAddress(page.0 + 0x1000);
        assert_eq!(mapper.translate(neighbour), None);
    }

    #[test]
    fn non_canonical_addresses_are_rejected() {
        let mut memory = FakePhysicalMemory::new();
        let mut allocator = allocator_over(&mut memory);
        let mut pml4 = PageTable::new();
        let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);

        // Bit 47 set but bits 48-63 clear. `split` discards the top 16
        // bits, so without the canonical check this would be mapped as if
        // it were 0xFFFF_8000_0000_0000 -- silently aliasing a
        // higher-half address that hardware would #GP on instead.
        let non_canonical = VirtualAddress(0x0000_8000_0000_0000);
        assert!(!non_canonical.is_canonical());
        assert!(VirtualAddress(0xFFFF_8000_0000_0000).is_canonical());
        assert!(VirtualAddress(0x0000_7FFF_FFFF_F000).is_canonical());

        let flags = PageFlags { present: true, writable: true, user_accessible: false };
        assert_eq!(
            mapper.map_page(non_canonical, PhysicalAddress(0x1000), flags, &mut allocator),
            Err(PageMappingError::InvalidAddress)
        );
        assert_eq!(mapper.unmap_page(non_canonical), Err(PageMappingError::InvalidAddress));
        assert_eq!(mapper.translate(non_canonical), None);
    }

    #[test]
    fn a_two_mib_leaf_is_resolved_and_never_walked_through() {
        let mut memory = FakePhysicalMemory::new();
        let mut allocator = allocator_over(&mut memory);
        let mut pml4 = PageTable::new();
        let page = VirtualAddress(0x0000_7FFF_FFFF_0000);
        let parts = page.split();
        let flags = PageFlags { present: true, writable: true, user_accessible: false };

        let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);
        // Build a real hierarchy first, then rewrite its PD entry into a
        // 2 MiB leaf -- which is what firmware would have handed us.
        assert_eq!(mapper.map_page(page, PhysicalAddress(0x1000_0000), flags, &mut allocator), Ok(()));

        // phys_offset is 0, so physical addresses are dereferenceable
        // directly. Reach down to the page directory and set PS.
        let huge_base = 0x4000_0000u64; // 1 GiB: also 2 MiB aligned.
        unsafe {
            let pdpt_phys = mapper.pml4.entry(parts.pml4_index).physical_address();
            let pdpt = &*(pdpt_phys.0 as *const PageTable);
            let pd_phys = pdpt.entry(parts.pdpt_index).physical_address();
            let pd = &mut *(pd_phys.0 as *mut PageTable);
            pd.set_entry(
                parts.pd_index,
                PageTableEntry::new(PhysicalAddress(huge_base), flags).marked_huge(),
            );
        }

        // Translation resolves the leaf with a 21-bit offset, rather than
        // reading the mapped data as if it were a page table.
        let expected = PhysicalAddress(huge_base + (page.0 & 0x1F_FFFF));
        assert_eq!(mapper.translate(page), Some(expected));

        // Mapping through it is refused. Before the PS check this wrote an
        // 8-byte entry into whatever the 2 MiB page maps -- silent
        // corruption of live memory, reported as success.
        assert_eq!(
            mapper.map_page(page, PhysicalAddress(0x2000_0000), flags, &mut allocator),
            Err(PageMappingError::HugePagePresent)
        );
        // And unmapping through it is refused rather than zeroing a word
        // of that page.
        assert_eq!(mapper.unmap_page(page), Err(PageMappingError::HugePagePresent));
    }

    #[test]
    fn intermediate_tables_only_allow_user_access_when_a_leaf_needs_it() {
        let mut memory = FakePhysicalMemory::new();
        let mut allocator = allocator_over(&mut memory);
        let mut pml4 = PageTable::new();
        let kernel_page = VirtualAddress(0x0000_7FFF_FFFF_0000);
        let parts = kernel_page.split();

        {
            let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);
            let kernel_flags =
                PageFlags { present: true, writable: true, user_accessible: false };
            assert_eq!(
                mapper.map_page(kernel_page, PhysicalAddress(0x1000_0000), kernel_flags, &mut allocator),
                Ok(())
            );
        }
        // A page is user-accessible only if U/S is set at every level, so
        // a blanket `user_accessible: true` on intermediates is how a
        // kernel page becomes reachable from ring 3.
        assert!(!pml4.entry(parts.pml4_index).is_user_accessible());

        // A user mapping that shares this PML4 entry widens it, otherwise
        // the user page would be unreachable through a table first built
        // for a kernel one.
        let user_page = VirtualAddress(kernel_page.0 + 0x1000);
        {
            let mut mapper = SoftwarePageMapper::new(&mut pml4, 0);
            let user_flags = PageFlags { present: true, writable: true, user_accessible: true };
            assert_eq!(
                mapper.map_page(user_page, PhysicalAddress(0x1000_1000), user_flags, &mut allocator),
                Ok(())
            );
        }
        assert!(pml4.entry(parts.pml4_index).is_user_accessible());
        // Widening must not disturb the frame the entry points at.
        assert!(pml4.entry(parts.pml4_index).is_present());
    }
}
