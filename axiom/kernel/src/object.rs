/// Kernel object model placeholder.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelObjectId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelObjectKind {
    Process,
    Thread,
    Channel,
    Event,
    Timer,
    SharedMemory,
    Device,
    AddressSpace,
    Unknown,
}
