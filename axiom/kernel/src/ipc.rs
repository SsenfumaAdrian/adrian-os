/// IPC scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHeader {
    pub message_id: u64,
    pub flags: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventObject {
    pub signaled: bool,
}

pub fn early_ipc_init() {
    // Planned:
    // - channel object model
    // - event object model
    // - shared memory model
    // - capability transfer semantics
}
