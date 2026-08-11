use crate::arch::x86_64::port_io::Port;

/// Programmable Interval Timer (8253/8254) frequency math and
/// channel-0 configuration.
///
/// The divisor calculation is pure arithmetic -- no port I/O. Actually
/// programming the hardware (mode byte, then the divisor's low/high
/// bytes) is a separate, explicitly gated step below.

/// The PIT's fixed input clock. A historical constant: derived from
/// the original IBM PC design's crystal frequency divided by 3.
pub const BASE_FREQUENCY_HZ: u32 = 1_193_182;

/// The largest count the PIT's 16-bit reload register can represent.
/// The register can't hold 65536 directly -- a written value of 0 is
/// interpreted by the hardware as 65536, its true maximum.
const MAX_DIVISOR: u32 = 65536;

const CHANNEL0_DATA: u16 = 0x40;
const COMMAND: u16 = 0x43;

/// Channel 0, lobyte/hibyte access, mode 3 (square wave generator) --
/// the standard configuration for a periodic tick interrupt.
const MODE_COMMAND_CHANNEL0_SQUARE_WAVE: u8 = 0x36;

/// Compute the reload value that makes the PIT tick at approximately
/// `desired_hz`. Integer division means the result is often not
/// exact -- `actual_frequency` reports what you really get back.
///
/// Returns `None` if `desired_hz` is zero, higher than the PIT's base
/// frequency can produce by dividing, or lower than the PIT can reach
/// even at its slowest setting (~18.2 Hz).
pub fn divisor_for_frequency(desired_hz: u32) -> Option<u16> {
    if desired_hz == 0 {
        return None;
    }

    let divisor = BASE_FREQUENCY_HZ / desired_hz;

    if divisor == 0 || divisor > MAX_DIVISOR {
        return None;
    }

    if divisor == MAX_DIVISOR {
        Some(0)
    } else {
        Some(divisor as u16)
    }
}

/// The actual tick frequency a given 16-bit reload value produces
/// (register encoding: 0 means 65536). Inverse of
/// `divisor_for_frequency`, useful for reporting how far off a
/// requested rate actually landed.
pub const fn actual_frequency(divisor: u16) -> u32 {
    let effective = if divisor == 0 { MAX_DIVISOR } else { divisor as u32 };
    BASE_FREQUENCY_HZ / effective
}

/// Program PIT channel 0 to tick at `divisor`'s rate and start it
/// running. `Port::write_u8` already no-ops under `--features std`,
/// same as the PIC remap -- no separate gating needed here either.
pub fn program_channel0(divisor: u16) {
    let command = Port::new(COMMAND);
    let data = Port::new(CHANNEL0_DATA);

    command.write_u8(MODE_COMMAND_CHANNEL0_SQUARE_WAVE);
    data.write_u8((divisor & 0xFF) as u8); // low byte first
    data.write_u8((divisor >> 8) as u8); // then high byte
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_frequency() {
        assert_eq!(divisor_for_frequency(0), None);
    }

    #[test]
    fn rejects_frequency_above_base_clock() {
        // The PIT only divides down; it can't produce a faster tick
        // than its own input clock.
        assert_eq!(divisor_for_frequency(BASE_FREQUENCY_HZ + 1), None);
        assert_eq!(divisor_for_frequency(2_000_000), None);
    }

    #[test]
    fn rejects_frequency_below_minimum_achievable_rate() {
        // The PIT's slowest reachable rate is BASE_FREQUENCY_HZ / 65536
        // (~18.2 Hz). 18 Hz's naive divisor overshoots the maximum
        // reload value, so there's no valid answer for it.
        assert_eq!(divisor_for_frequency(18), None);
    }

    #[test]
    fn accepts_lowest_representable_frequency() {
        assert_eq!(divisor_for_frequency(19), Some(62799));
    }

    #[test]
    fn divisor_for_1000hz_matches_known_value() {
        assert_eq!(divisor_for_frequency(1000), Some(1193));
    }

    #[test]
    fn divisor_for_100hz_matches_known_value() {
        assert_eq!(divisor_for_frequency(100), Some(11931));
    }

    #[test]
    fn actual_frequency_is_close_to_requested() {
        let divisor = divisor_for_frequency(1000).unwrap();
        let actual = actual_frequency(divisor);
        assert!(actual >= 995 && actual <= 1005, "got {}", actual);
    }

    #[test]
    fn zero_divisor_means_65536_per_datasheet_convention() {
        assert_eq!(actual_frequency(0), BASE_FREQUENCY_HZ / 65536);
    }

    #[test]
    fn slowest_possible_rate_matches_classic_pc_tick() {
        // The historical default PC timer rate (~18.2 Hz) is exactly
        // what divisor 0 (== 65536, the PIT's max) produces.
        let lowest = actual_frequency(0);
        assert!(lowest >= 18 && lowest <= 19, "got {}", lowest);
    }

    #[test]
    fn program_channel0_does_not_panic_on_the_hosted_path() {
        program_channel0(divisor_for_frequency(1000).unwrap());
    }
}
