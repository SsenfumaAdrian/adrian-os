#!/bin/sh
# Boot assertions for the Rian bare-metal image.
#
#   sh tools/image/verify_boot.sh serial.log
#
# The companion to verify_shape.sh, one verification level up. Shape says
# the image is loadable; this says the machine actually loaded it, entered
# long mode, reached Rust, ran the whole init sequence in order, and parked
# deliberately rather than dying. That is the difference between
# `compiled` and `boot tested (emulated)` in PROGRESS.md's vocabulary, and
# this script is what earns the second label.
#
# It reads a serial log rather than a QEMU exit code on purpose. The image
# ends in `halt_forever()` and contains no `isa-debug-exit` write, because
# a port write whose only purpose is to tell a test harness the answer is
# test-only code living inside the artifact that ships. So the harness
# reads what the kernel says, and the kernel says it on COM1 whether or
# not anyone is listening.
#
# Every assertion here is about a string the kernel emits today:
# `handoff_label` in rian/bare-metal/src/main.rs, `BootStage::label` in
# rian/kernel/src/boot_trace.rs, and the two markers in
# rian/kernel/src/entry.rs. If a label is renamed, this fails -- which is
# the point: the boot log is a wire format, and boot_trace.rs says so.
#
# All twelve checks have been shown to fail when the thing they assert is
# actually broken, against hand-built logs: empty, a bare `C` and a bare
# `L`, each of the five failure markers, a stage removed, a stage
# reordered, and the handoff replaced by each of its other two outcomes.
# Both an LF log and a CRLF log pass 12/12.
#
# One warning for anyone repeating that pass, learned by getting it wrong:
# build the perturbation from the string the source actually contains, not
# from a plausible spelling of it. `panic.rs` writes `RIAN PANIC` with no
# colon, and a hand-written log saying `RIAN: PANIC: memory` leaves the
# panic check green -- which looks like a broken check and is really a
# broken perturbation. `grep -rn RIAN rian/kernel/src rian/bare-metal/src`
# is the authoritative list.

set -u

LOG="${1:-}"
if [ -z "$LOG" ]; then
    echo "usage: sh tools/image/verify_boot.sh <serial-log>" >&2
    exit 2
fi
if [ ! -f "$LOG" ]; then
    echo "no such log: $LOG" >&2
    exit 2
fi

# QEMU's serial output arrives with CRLF, since serial_debug_write_line
# writes both. Normalize once here so every pattern below can anchor.
PLAIN=$(tr -d '\r' < "$LOG")

PASSED=0
FAILED=0

check() {
    if [ "$2" = "1" ]; then
        PASSED=$((PASSED + 1))
        printf 'ok   %s\n' "$1"
    else
        FAILED=$((FAILED + 1))
        printf 'FAIL %s\n' "$1"
        if [ -n "${3:-}" ]; then
            printf '     %s\n' "$3"
        fi
    fi
}

yes_if() {
    if [ "$1" = "$2" ]; then echo 1; else echo 0; fi
}

has() {
    if printf '%s\n' "$PLAIN" | grep -qF "$1"; then echo 1; else echo 0; fi
}

echo "serial log: $LOG ($(printf '%s\n' "$PLAIN" | grep -c . ) non-empty lines)"
echo

# ---------------------------------------------------------------------
# 1. Something came out at all
# ---------------------------------------------------------------------
#
# The failure this catches is the only one a boot can have with no
# diagnostic: a triple fault during the long-mode transition resets the
# machine before a single byte reaches COM1. An empty log is therefore not
# "the test could not tell" -- it is the specific, expected signature of
# the entry stub going wrong, and it has to be a hard failure rather than
# a skipped assertion.
lines=$(printf '%s\n' "$PLAIN" | grep -c .)
check "the serial log is not empty" \
    "$(yes_if "$([ "$lines" -gt 0 ] && echo y || echo n)" "y")" \
    "nothing on COM1; the usual cause is a fault before rian_main, which resets with no output"

# ---------------------------------------------------------------------
# 2. The entry stub did not reject the machine
# ---------------------------------------------------------------------
#
# boot.s writes a single character and halts: 'C' for no CPUID extended
# leaf, 'L' for no long mode. Matched as whole lines, because a bare C or
# L on its own line is what boot_fail produces and nothing else in the log
# is one character wide.
check "the entry stub did not report a missing CPUID leaf" \
    "$(yes_if "$(printf '%s\n' "$PLAIN" | grep -c '^C$')" "0")" \
    "boot.s wrote 'C': CPUID has no 0x80000001 leaf on this CPU model"
check "the entry stub did not report a missing long mode" \
    "$(yes_if "$(printf '%s\n' "$PLAIN" | grep -c '^L$')" "0")" \
    "boot.s wrote 'L': CPUID reports no long mode on this CPU model"

# ---------------------------------------------------------------------
# 3. Rust ran, and the handoff was the one the image was built for
# ---------------------------------------------------------------------
#
# Asserting the *specific* multiboot1 line, not merely that some handoff
# line appeared. `handoff_label` has three outcomes and two of them mean
# the image was loaded by something that did not set up what it expects;
# under `qemu -kernel` the third is the only correct one, so accepting any
# of them would turn a real regression into a pass.
check "rian_main reported a multiboot1 handoff" \
    "$(has 'RIAN: multiboot1 handoff')" \
    "no handoff line; either Rust never ran, or the UART was not up when it did"
check "the handoff carried an information structure" \
    "$(yes_if "$(has 'RIAN: multiboot1 handoff with no information structure')" "0")" \
    "ebx was 0 at _start; the memory map roadmap step 4 needs would not be there"
check "the handoff magic was recognized" \
    "$(yes_if "$(has 'handoff magic unrecognized')" "0")" \
    "eax was not 0x2BADB002; the loader was not multiboot1-compliant"

# ---------------------------------------------------------------------
# 4. Init ran every stage, in order
# ---------------------------------------------------------------------
#
# The ordering assertion is the one worth having. Any single stage can be
# grepped for individually and all ten can be present while the sequence
# is wrong -- a re-entered init, a stage recorded twice, memory brought up
# before arch. So the stage lines are extracted in log order and compared
# to the expected sequence as one string, which also catches duplicates
# and omissions in the same comparison.
EXPECTED='entry boot-context arch memory security ipc scheduler process thread idle'
observed=$(printf '%s\n' "$PLAIN" | awk '
    /^(entry|boot-context|arch|memory|security|ipc|scheduler|process|thread|idle)$/ {
        printf "%s%s", sep, $0; sep = " "
    }
    END { print "" }')
check "the boot trace records all ten stages in order" \
    "$(yes_if "$observed" "$EXPECTED")" \
    "expected: $EXPECTED
     observed: ${observed:-<no stage lines at all>}"

# ---------------------------------------------------------------------
# 5. Init succeeded and the kernel parked on purpose
# ---------------------------------------------------------------------

check "init did not reject the boot context" \
    "$(yes_if "$(has 'RIAN: INVALID BOOT CONTEXT')" "0")" \
    "BootContext::is_valid() failed on a context this crate built itself"
check "the bootstrap process was created" \
    "$(yes_if "$(has 'RIAN: BOOTSTRAP PROCESS CREATION FAILED')" "0")" ""
check "the bootstrap thread was created" \
    "$(yes_if "$(has 'RIAN: BOOTSTRAP THREAD CREATION FAILED')" "0")" ""
check "nothing panicked" \
    "$(yes_if "$(has 'RIAN PANIC')" "0")" \
    "the #[panic_handler] ran; on bare metal that is a real fault, not a test failure"
check "init completed and the kernel halted deliberately" \
    "$(has 'RIAN: INIT COMPLETE, HALTING')" \
    "no completion marker; init returned something other than Ready, or never returned"

echo
if [ "$FAILED" -eq 0 ]; then
    printf 'boot: %d/%d checks passed\n' "$PASSED" "$((PASSED + FAILED))"
    exit 0
fi
printf 'boot: %d/%d checks passed, %d FAILED\n' "$PASSED" "$((PASSED + FAILED))" "$FAILED"
exit 1
