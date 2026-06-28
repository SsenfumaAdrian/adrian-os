/// IPC scaffold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Open,
    Closed,
}

pub fn early_ipc_init() {
    // Planned:
    // - channel object model
    // - event object model
    // - shared memory primitives
    // - handle transfer support
}
