/// IPC scaffold.
use crate::error::KernelError;

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
    signaled: bool,
}

impl EventObject {
    pub const fn new() -> Self {
        Self { signaled: false }
    }

    pub fn signal(&mut self) {
        self.signaled = true;
    }

    /// Manual reset back to unsignaled. There's no auto-reset-on-wait
    /// variant here -- that's a real wait/wake distinction that needs
    /// actual thread blocking to mean anything, which doesn't exist
    /// yet (same reason waiting is polling below, not blocking).
    pub fn clear(&mut self) {
        self.signaled = false;
    }

    pub const fn is_signaled(&self) -> bool {
        self.signaled
    }
}

/// Fixed maximum payload size. Small enough to be a completely
/// reasonable static allocation, large enough to carry more than a
/// bare token value. A real channel would want variable-length or
/// scatter-gather payloads; no heap allocator exists yet to build
/// that on, so a fixed cap is the honest starting point.
pub const MAX_MESSAGE_PAYLOAD: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Message {
    pub header: MessageHeader,
    payload: [u8; MAX_MESSAGE_PAYLOAD],
    payload_len: usize,
}

impl Message {
    /// `None` if `payload` is longer than `MAX_MESSAGE_PAYLOAD`.
    pub fn new(header: MessageHeader, payload: &[u8]) -> Option<Self> {
        if payload.len() > MAX_MESSAGE_PAYLOAD {
            return None;
        }
        let mut buffer = [0u8; MAX_MESSAGE_PAYLOAD];
        buffer[..payload.len()].copy_from_slice(payload);
        Some(Self {
            header,
            payload: buffer,
            payload_len: payload.len(),
        })
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.payload_len]
    }
}

/// Fixed-capacity queued-message limit, for the same reason as
/// MAX_MESSAGE_PAYLOAD: no heap exists yet for anything unbounded.
pub const MAX_QUEUED_MESSAGES: usize = 8;

/// A channel: an ordered, fixed-capacity message queue with open/
/// closed state. Array-backed ring buffer, same underlying pattern as
/// sched::RunQueue -- deliberately not shared code with it though;
/// duplicating roughly a dozen lines of proven-correct logic is a
/// smaller risk right now than refactoring already-tested code to
/// generalize over it.
#[derive(Clone, Copy)]
pub struct Channel {
    state: ChannelState,
    messages: [Option<Message>; MAX_QUEUED_MESSAGES],
    head: usize,
    len: usize,
}

impl Channel {
    pub const fn new() -> Self {
        Self {
            state: ChannelState::Open,
            messages: [None; MAX_QUEUED_MESSAGES],
            head: 0,
            len: 0,
        }
    }

    pub const fn state(&self) -> ChannelState {
        self.state
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == MAX_QUEUED_MESSAGES
    }

    /// Queue a message. `Busy` if the queue is full -- a transient
    /// condition a receiver draining the channel resolves, so this is
    /// backpressure, not a hard failure. `Closed` if the channel has
    /// been closed; there's nowhere to grow into either way (no heap),
    /// so a full channel can't just expand instead of rejecting.
    pub fn send(&mut self, message: Message) -> Result<(), KernelError> {
        if self.state == ChannelState::Closed {
            return Err(KernelError::Closed);
        }
        if self.is_full() {
            return Err(KernelError::Busy);
        }
        let tail = (self.head + self.len) % MAX_QUEUED_MESSAGES;
        self.messages[tail] = Some(message);
        self.len += 1;
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Message> {
        if self.is_empty() {
            return None;
        }
        let message = self.messages[self.head].take();
        self.head = (self.head + 1) % MAX_QUEUED_MESSAGES;
        self.len -= 1;
        message
    }

    /// Close the channel. Already-queued messages can still be
    /// drained with `receive` afterward -- closing stops new sends,
    /// it doesn't discard what's already in flight.
    pub fn close(&mut self) {
        self.state = ChannelState::Closed;
    }
}

/// Fixed-capacity channel registry. Stores (id, Channel) pairs rather
/// than adding an id field to Channel itself -- keeps Channel unaware
/// of its own registry identity (the table owns that mapping, the
/// object doesn't need to), and avoids touching Channel's existing
/// constructor/tests for something only the table needs.
pub const MAX_CHANNELS: usize = 16;

pub struct ChannelTable<const CAPACITY: usize> {
    slots: [Option<(crate::object::KernelObjectId, Channel)>; CAPACITY],
}

impl<const CAPACITY: usize> ChannelTable<CAPACITY> {
    pub const fn new() -> Self {
        Self { slots: [None; CAPACITY] }
    }

    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Create a new open channel. `None` if the table or the global
    /// handle registry is full.
    pub fn create(&mut self) -> Option<crate::object::KernelObjectId> {
        let slot = self.slots.iter().position(|s| s.is_none())?;
        let id = crate::object::allocate_id(crate::object::KernelObjectKind::Channel)?;
        self.slots[slot] = Some((id, Channel::new()));
        Some(id)
    }

    pub fn get_mut(&mut self, id: crate::object::KernelObjectId) -> Option<&mut Channel> {
        self.slots
            .iter_mut()
            .flatten()
            .find(|(entry_id, _)| *entry_id == id)
            .map(|(_, channel)| channel)
    }

    /// Remove a channel from the table entirely -- distinct from
    /// `Channel::close`, which marks it Closed while leaving already-
    /// queued messages drainable. This actually frees the slot, and
    /// unregisters the id from the global handle registry in the same
    /// step, mirroring how `create` bundles allocation with
    /// registration rather than leaving it as a separate call a
    /// caller could forget.
    pub fn destroy(&mut self, id: crate::object::KernelObjectId) -> bool {
        match self
            .slots
            .iter_mut()
            .position(|s| matches!(s, Some((entry_id, _)) if *entry_id == id))
        {
            Some(index) => {
                self.slots[index] = None;
                crate::object::HANDLE_REGISTRY.lock().unregister(id);
                true
            }
            None => false,
        }
    }
}

/// The kernel's channel registry, same pattern as PROCESS_TABLE and
/// THREAD_TABLE.
pub static CHANNEL_TABLE: crate::sync::SpinLock<ChannelTable<MAX_CHANNELS>> =
    crate::sync::SpinLock::new(ChannelTable::new());

/// General entry points a real ChannelCreate/HandleClose syscall
/// handler calls, matching process::create_process /
/// thread::create_thread's naming.
pub fn create_channel() -> Option<crate::object::KernelObjectId> {
    CHANNEL_TABLE.lock().create()
}

pub fn destroy_channel(id: crate::object::KernelObjectId) -> bool {
    CHANNEL_TABLE.lock().destroy(id)
}

/// Fixed-capacity event registry, same shape as ChannelTable.
pub const MAX_EVENTS: usize = 16;

pub struct EventTable<const CAPACITY: usize> {
    slots: [Option<(crate::object::KernelObjectId, EventObject)>; CAPACITY],
}

impl<const CAPACITY: usize> EventTable<CAPACITY> {
    pub const fn new() -> Self {
        Self { slots: [None; CAPACITY] }
    }

    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn create(&mut self) -> Option<crate::object::KernelObjectId> {
        let slot = self.slots.iter().position(|s| s.is_none())?;
        let id = crate::object::allocate_id(crate::object::KernelObjectKind::Event)?;
        self.slots[slot] = Some((id, EventObject::new()));
        Some(id)
    }

    pub fn get_mut(&mut self, id: crate::object::KernelObjectId) -> Option<&mut EventObject> {
        self.slots
            .iter_mut()
            .flatten()
            .find(|(entry_id, _)| *entry_id == id)
            .map(|(_, event)| event)
    }

    pub fn destroy(&mut self, id: crate::object::KernelObjectId) -> bool {
        match self
            .slots
            .iter_mut()
            .position(|s| matches!(s, Some((entry_id, _)) if *entry_id == id))
        {
            Some(index) => {
                self.slots[index] = None;
                crate::object::HANDLE_REGISTRY.lock().unregister(id);
                true
            }
            None => false,
        }
    }
}

pub static EVENT_TABLE: crate::sync::SpinLock<EventTable<MAX_EVENTS>> =
    crate::sync::SpinLock::new(EventTable::new());

pub fn create_event() -> Option<crate::object::KernelObjectId> {
    EVENT_TABLE.lock().create()
}

pub fn destroy_event(id: crate::object::KernelObjectId) -> bool {
    EVENT_TABLE.lock().destroy(id)
}

/// Not wired to any syscall number yet -- SyscallNumber only has
/// EventCreate, no EventSignal/EventWait. Available for other kernel
/// code to call directly in the meantime, same as Channel's
/// send/receive: create/destroy is the handle-lifecycle concern this
/// commit completes, actual I/O on the handle is separately still
/// open, for events same as it already was for channels.
pub fn signal_event(id: crate::object::KernelObjectId) -> bool {
    match EVENT_TABLE.lock().get_mut(id) {
        Some(event) => {
            event.signal();
            true
        }
        None => false,
    }
}

pub fn is_event_signaled(id: crate::object::KernelObjectId) -> Option<bool> {
    EVENT_TABLE.lock().get_mut(id).map(|event| event.is_signaled())
}

pub fn early_ipc_init() {
    // Channel and Message have real send/receive logic, and both
    // ChannelTable and EventTable now give channels and events a real
    // syscall-reachable home (create_channel/destroy_channel,
    // create_event/destroy_event) -- all proven correct by this
    // module's own tests rather than exercised here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: u64) -> MessageHeader {
        MessageHeader {
            message_id: id,
            flags: 0,
            reserved: 0,
        }
    }

    #[test]
    fn message_new_rejects_payload_too_large() {
        let payload = [0u8; MAX_MESSAGE_PAYLOAD + 1];
        assert!(Message::new(header(1), &payload).is_none());
    }

    #[test]
    fn message_new_accepts_payload_at_the_limit() {
        let payload = [7u8; MAX_MESSAGE_PAYLOAD];
        let message = Message::new(header(1), &payload).unwrap();
        assert_eq!(message.payload(), &payload[..]);
    }

    #[test]
    fn message_payload_round_trips_exactly() {
        let payload = [1u8, 2, 3, 4, 5];
        let message = Message::new(header(1), &payload).unwrap();
        assert_eq!(message.payload(), &payload[..]);
    }

    #[test]
    fn new_channel_is_open_and_empty() {
        let channel = Channel::new();
        assert_eq!(channel.state(), ChannelState::Open);
        assert!(channel.is_empty());
    }

    #[test]
    fn send_then_receive_preserves_fifo_order() {
        let mut channel = Channel::new();
        channel.send(Message::new(header(1), &[]).unwrap()).unwrap();
        channel.send(Message::new(header(2), &[]).unwrap()).unwrap();

        assert_eq!(channel.receive().unwrap().header.message_id, 1);
        assert_eq!(channel.receive().unwrap().header.message_id, 2);
    }

    #[test]
    fn receive_on_empty_channel_returns_none() {
        let mut channel = Channel::new();
        assert_eq!(channel.receive(), None);
    }

    #[test]
    fn send_fails_when_channel_is_full() {
        let mut channel = Channel::new();
        for i in 0..MAX_QUEUED_MESSAGES as u64 {
            channel.send(Message::new(header(i), &[]).unwrap()).unwrap();
        }
        assert!(channel.is_full());
        let result = channel.send(Message::new(header(999), &[]).unwrap());
        assert_eq!(result, Err(KernelError::Busy));
    }

    #[test]
    fn send_fails_on_closed_channel() {
        let mut channel = Channel::new();
        channel.close();
        let result = channel.send(Message::new(header(1), &[]).unwrap());
        assert_eq!(result, Err(KernelError::Closed));
    }

    #[test]
    fn closing_does_not_discard_already_queued_messages() {
        let mut channel = Channel::new();
        channel.send(Message::new(header(1), &[]).unwrap()).unwrap();
        channel.close();
        assert_eq!(channel.receive().unwrap().header.message_id, 1);
    }

    #[test]
    fn ring_buffer_wraps_around_correctly() {
        // MAX_QUEUED_MESSAGES is 8. Fill completely, drain the first
        // half, refill past the array's physical end, then confirm
        // FIFO order held across the wrap -- hand-verified before
        // trusting it, same discipline as sched::RunQueue's
        // equivalent test.
        let mut channel = Channel::new();

        for i in 0..8u64 {
            channel.send(Message::new(header(i), &[]).unwrap()).unwrap();
        }
        assert!(channel.is_full());

        for i in 0..4u64 {
            assert_eq!(channel.receive().unwrap().header.message_id, i);
        }

        for i in 8..12u64 {
            channel.send(Message::new(header(i), &[]).unwrap()).unwrap();
        }
        assert!(channel.is_full());

        for i in 4..12u64 {
            assert_eq!(channel.receive().unwrap().header.message_id, i);
        }
        assert!(channel.is_empty());
    }

    #[test]
    fn table_create_returns_a_valid_nonzero_id() {
        // Touches the real global allocate_id/HANDLE_REGISTRY, same
        // as any other test creating a process/thread/channel
        // concurrently -- only asserting a relative property (not the
        // reserved sentinel), safe regardless of execution order.
        let mut table: ChannelTable<4> = ChannelTable::new();
        let id = table.create();
        assert!(id.is_some());
        assert_ne!(id.unwrap().0, 0);
    }

    #[test]
    fn table_create_fails_when_full() {
        let mut table: ChannelTable<2> = ChannelTable::new();
        assert!(table.create().is_some());
        assert!(table.create().is_some());
        assert!(table.create().is_none());
        assert_eq!(table.count(), 2);
    }

    #[test]
    fn table_get_mut_finds_the_right_channel_and_none_for_unknown() {
        let mut table: ChannelTable<4> = ChannelTable::new();
        let id = table.create().unwrap();

        table
            .get_mut(id)
            .unwrap()
            .send(Message::new(header(1), &[]).unwrap())
            .unwrap();
        assert_eq!(table.get_mut(id).unwrap().receive().unwrap().header.message_id, 1);

        assert!(table.get_mut(crate::object::KernelObjectId(u64::MAX)).is_none());
    }

    #[test]
    fn table_destroy_frees_the_slot_and_unregisters_the_id() {
        let mut table: ChannelTable<2> = ChannelTable::new();
        let id = table.create().unwrap();
        assert_eq!(table.count(), 1);

        assert!(table.destroy(id));
        assert_eq!(table.count(), 0);
        assert!(table.get_mut(id).is_none());
        assert_eq!(crate::object::HANDLE_REGISTRY.lock().kind_of(id), None);

        // A destroyed id can't be destroyed again.
        assert!(!table.destroy(id));
    }

    #[test]
    fn new_event_starts_unsignaled() {
        assert!(!EventObject::new().is_signaled());
    }

    #[test]
    fn signal_then_clear_round_trips() {
        let mut event = EventObject::new();
        event.signal();
        assert!(event.is_signaled());
        event.clear();
        assert!(!event.is_signaled());
    }

    #[test]
    fn event_table_create_returns_a_valid_nonzero_id() {
        let mut table: EventTable<4> = EventTable::new();
        let id = table.create();
        assert!(id.is_some());
        assert_ne!(id.unwrap().0, 0);
    }

    #[test]
    fn event_table_create_fails_when_full() {
        let mut table: EventTable<2> = EventTable::new();
        assert!(table.create().is_some());
        assert!(table.create().is_some());
        assert!(table.create().is_none());
        assert_eq!(table.count(), 2);
    }

    #[test]
    fn event_table_get_mut_finds_the_right_event_and_none_for_unknown() {
        let mut table: EventTable<4> = EventTable::new();
        let id = table.create().unwrap();

        table.get_mut(id).unwrap().signal();
        assert!(table.get_mut(id).unwrap().is_signaled());

        assert!(table.get_mut(crate::object::KernelObjectId(u64::MAX)).is_none());
    }

    #[test]
    fn event_table_destroy_frees_the_slot_and_unregisters_the_id() {
        let mut table: EventTable<2> = EventTable::new();
        let id = table.create().unwrap();
        assert_eq!(table.count(), 1);

        assert!(table.destroy(id));
        assert_eq!(table.count(), 0);
        assert!(table.get_mut(id).is_none());
        assert_eq!(crate::object::HANDLE_REGISTRY.lock().kind_of(id), None);

        assert!(!table.destroy(id));
    }

    #[test]
    fn global_signal_and_check_round_trip_through_the_real_table() {
        let id = create_event().unwrap();
        assert_eq!(is_event_signaled(id), Some(false));
        assert!(signal_event(id));
        assert_eq!(is_event_signaled(id), Some(true));
        destroy_event(id);
        assert_eq!(is_event_signaled(id), None);
    }
}
