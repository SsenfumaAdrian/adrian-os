/// x86_64 4-level paging (PML4 -> PDPT -> PD -> PT).
///
/// Entry encoding and address splitting are pure data manipulation --
/// fully verifiable without hardware, same as idt.rs's entry encoding.
/// Actually pointing CR3 at a table and flipping on paging is real
/// privileged state with no way to test it outside real or
/// virtualized hardware, so that step isn't attempted here. Neither is
/// a table-walking mapper: whether physical addresses are directly
/// dereferenceable while building tables depends on the boot-time
/// memory model (is paging already on from firmware by the time this
/// runs, identity-mapped, or something else), which depends on real
/// firmware/Halo integration that doesn't exist yet. Building a mapper
/// on top of an assumption that isn't settled would look more real
/// than it is.
use crate::mm::{PhysicalAddress, VirtualAddress};

const PRESENT_BIT: u64 = 1 << 0;
const WRITABLE_BIT: u64 = 1 << 1;
const USER_BIT: u64 = 1 << 2;
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
}

/// Software-driven 4-level page table mapper (PML4 -> PDPT -> PD -> PT).
/// Operates on a root PML4 table, walking and creating intermediate tables as needed.
pub struct SoftwarePageMapper<'a> {
    pml4: &'a mut PageTable,
    /// Physical memory offset mapping physical addresses to virtual accessibility pointers.
    phys_offset: u64,
}

impl<'a> SoftwarePageMapper<'a> {
    pub fn new(pml4: &'a mut PageTable, phys_offset: u64) -> Self {
        Self { pml4, phys_offset }
    }

    /// Convert a physical address to a mutable reference to a `PageTable` using `phys_offset`.
    ///
    /// # Safety
    /// Caller must ensure `phys` points to a valid `PageTable` allocation.
    unsafe fn phys_to_table_mut(&self, phys: PhysicalAddress) -> &'a mut PageTable {
        let virt_ptr = (phys.0 + self.phys_offset) as *mut PageTable;
        &mut *virt_ptr
    }

    /// Convert a physical address to an immutable reference to a `PageTable` using `phys_offset`.
    ///
    /// # Safety
    /// Caller must ensure `phys` points to a valid `PageTable` allocation.
    unsafe fn phys_to_table(&self, phys: PhysicalAddress) -> &'a PageTable {
        let virt_ptr = (phys.0 + self.phys_offset) as *const PageTable;
        &*virt_ptr
    }

    /// Walk the 4-level hierarchy and translate a `VirtualAddress` to its mapped `PhysicalAddress`.
    pub fn translate(&self, virt: VirtualAddress) -> Option<PhysicalAddress> {
        let parts = virt.split();

        let pml4e = self.pml4.entry(parts.pml4_index);
        if !pml4e.is_present() {
            return None;
        }

        let pdpt = unsafe { self.phys_to_table(pml4e.physical_address()) };
        let pdpte = pdpt.entry(parts.pdpt_index);
        if !pdpte.is_present() {
            return None;
        }

        let pd = unsafe { self.phys_to_table(pdpte.physical_address()) };
        let pde = pd.entry(parts.pd_index);
        if !pde.is_present() {
            return None;
        }

        let pt = unsafe { self.phys_to_table(pde.physical_address()) };
        let pte = pt.entry(parts.pt_index);
        if !pte.is_present() {
            return None;
        }

        Some(PhysicalAddress(pte.physical_address().0 + parts.offset as u64))
    }

    /// Map a virtual page to a physical frame, allocating intermediate page tables via `allocator` as needed.
    pub fn map_page(
        &mut self,
        virt: VirtualAddress,
        phys: PhysicalAddress,
        flags: PageFlags,
        allocator: &mut crate::mm::BootstrapAllocator,
    ) -> Result<(), PageMappingError> {
        let parts = virt.split();

        // 1. PML4 -> PDPT
        let pdpt_phys = Self::get_or_create_table_at(self.pml4, parts.pml4_index, self.phys_offset, allocator)?;
        let pdpt = unsafe { self.phys_to_table_mut(pdpt_phys) };

        // 2. PDPT -> PD
        let pd_phys = Self::get_or_create_table_at(pdpt, parts.pdpt_index, self.phys_offset, allocator)?;
        let pd = unsafe { self.phys_to_table_mut(pd_phys) };

        // 3. PD -> PT
        let pt_phys = Self::get_or_create_table_at(pd, parts.pd_index, self.phys_offset, allocator)?;
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
    pub fn unmap_page(&mut self, virt: VirtualAddress) -> Result<PhysicalAddress, PageMappingError> {
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

        let pd = unsafe { self.phys_to_table_mut(pdpte.physical_address()) };
        let pde = pd.entry(parts.pd_index);
        if !pde.is_present() {
            return Err(PageMappingError::NotMapped);
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

    fn get_or_create_table_at(
        parent: &mut PageTable,
        index: usize,
        phys_offset: u64,
        allocator: &mut crate::mm::BootstrapAllocator,
    ) -> Result<PhysicalAddress, PageMappingError> {
        let entry = parent.entry(index);
        if entry.is_present() {
            Ok(entry.physical_address())
        } else {
            let new_table_phys = allocator
                .allocate(4096, 4096)
                .ok_or(PageMappingError::AllocationFailed)?;

            let virt_ptr = (new_table_phys.0 + phys_offset) as *mut PageTable;
            let new_table = unsafe { &mut *virt_ptr };
            *new_table = PageTable::new();

            parent.set_entry(
                index,
                PageTableEntry::new(
                    new_table_phys,
                    PageFlags {
                        present: true,
                        writable: true,
                        user_accessible: true,
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
}
