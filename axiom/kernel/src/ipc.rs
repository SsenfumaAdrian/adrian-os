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
    pub signaled: bool,
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

pub fn early_ipc_init() {
    // Channel and Message now have real send/receive logic, proven
    // correct by this module's own tests rather than exercised here.
    // No global channel table to seed: unlike the scheduler's single
    // ready queue, channels are created per-use (a real ChannelCreate
    // syscall -- still a stub in syscall.rs, since wiring it needs a
    // handle table that doesn't exist yet), not pre-allocated at boot.
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
}
