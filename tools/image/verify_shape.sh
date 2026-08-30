#!/bin/sh
# Shape assertions for the Rian bare-metal image.
#
#   sh tools/image/verify_shape.sh rian/bare-metal/target/x86_64-unknown-none/release/rian
#
# A successful `cargo build` says the code compiles and links. It says
# nothing about whether the thing that came out is loadable, and for a
# bare-metal artifact almost every way of being unloadable is silent: an
# ELF of type DYN whose relocations nobody will apply, a multiboot header
# that section-garbage-collection removed, a load address that is not
# where the bootloader will put it, a page table that lost its 4 KiB
# alignment. Each of those produces a working build and a machine that
# resets with no output.
#
# So these checks sit between "it built" and "it booted". They are the
# cheapest verification level this project has for the image -- they need
# no emulator, no hardware, and about a second -- and they are the only
# ones that still work when the QEMU boot test cannot run.
#
# Written in POSIX sh against readelf and od on purpose: those exist in
# every CI image that can build Rust, whereas llvm-tools and gdb do not.
# Invoked as `sh tools/image/verify_shape.sh` so the executable bit is
# irrelevant, which matters because the working tree it is committed from
# is Windows.
#
# Every check below has been shown to fail when the thing it asserts is
# actually broken, which is the only evidence that a green run means
# anything. The perturbations used, against an image assembled with GNU as
# and linked with GNU ld from the real boot.s and rian.ld: discarding the
# multiboot header section, moving the load address to 0x200000, AT() to
# split LMA from VMA, ENTRY() pointed elsewhere, a padding byte ahead of
# boot_pml4, the stack shrunk to 4 KiB, the stack top misaligned by 8, the
# reserved memory emitted as stored bytes, a wrong checksum literal,
# `strip`, a 32-bit link, e_type patched to DYN, a real shared object,
# rian_main left undefined, and `ld -q`. Each turned exactly the expected
# check red -- and the pass found one bug, noted at the boot_pml4 check.

set -u

IMAGE="${1:-}"
if [ -z "$IMAGE" ]; then
    echo "usage: sh tools/image/verify_shape.sh <path-to-image-elf>" >&2
    exit 2
fi
if [ ! -f "$IMAGE" ]; then
    echo "no such image: $IMAGE" >&2
    exit 2
fi

PASSED=0
FAILED=0

# `condition` is the string 1 or 0 rather than an exit status, so that
# every call site reads as an assertion about a value it computed and
# printed, instead of as a command whose success is incidental.
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

# Address of a symbol, as the raw hex readelf prints. Empty if absent.
sym() {
    readelf -sW "$IMAGE" | awk -v want="$1" '$8 == want { print $2; exit }'
}

echo "image: $IMAGE"
echo

# ---------------------------------------------------------------------
# 1. It is the right kind of file
# ---------------------------------------------------------------------

header=$(readelf -hW "$IMAGE")

class=$(printf '%s\n' "$header" | awk -F': *' '/^ *Class:/ { print $2 }')
check "ELF class is ELF64" "$(yes_if "$class" "ELF64")" "class was '$class'"

machine=$(printf '%s\n' "$header" | awk -F': *' '/^ *Machine:/ { print $2 }')
check "machine is x86-64" \
    "$(yes_if "$machine" "Advanced Micro Devices X86-64")" \
    "machine was '$machine'"

# EXEC, not DYN. `x86_64-unknown-none` defaults to position-independent
# executables, and a bootloader does not run a dynamic loader -- so if
# `-C relocation-model=static` ever stops taking effect, this is the check
# that notices, and it notices before the machine silently fails to boot.
kind=$(printf '%s\n' "$header" | awk -F': *' '/^ *Type:/ { print $2 }')
check "ELF type is EXEC, not a position-independent DYN" \
    "$(yes_if "$kind" "EXEC (Executable file)")" \
    "type was '$kind'"

# ---------------------------------------------------------------------
# 2. It starts where it says it starts
# ---------------------------------------------------------------------

entry=$(printf '%s\n' "$header" | awk -F': *' '/Entry point address:/ { print $2 }')
start=$(sym _start)
check "_start exists in the symbol table" \
    "$(yes_if "$([ -n "$start" ] && echo y || echo n)" "y")" \
    "no _start; the linker script's ENTRY() had nothing to point at"
check "the ELF entry point is _start" \
    "$(yes_if "$((entry))" "$((0x0${start:-0}))")" \
    "entry $entry, _start 0x${start:-<missing>}"

# The assembly ends in `call rian_main`. If the Rust side ever stops
# exporting that symbol under that name -- a missing `#[no_mangle]`, a
# rename -- the link fails, so this check is about something subtler:
# confirming the two halves of this crate are actually joined, rather than
# the image having been produced by some path that skipped the Rust.
check "rian_main is defined" \
    "$(yes_if "$([ -n "$(sym rian_main)" ] && echo y || echo n)" "y")" \
    "no rian_main symbol"

# ---------------------------------------------------------------------
# 3. It loads where a multiboot loader will put it
# ---------------------------------------------------------------------

load=$(readelf -lW "$IMAGE" | awk '$1 == "LOAD" { print $3, $4; exit }')
virt=$(printf '%s\n' "$load" | awk '{ print $1 }')
phys=$(printf '%s\n' "$load" | awk '{ print $2 }')

check "the first LOAD segment is at 1 MiB" \
    "$(yes_if "$((${virt:-0}))" "$((0x100000))")" \
    "VirtAddr was ${virt:-<no LOAD segment>}"

# Paging is off when the bootloader copies the image, so it honours
# PhysAddr; the entry stub then identity-maps, so virtual must equal
# physical or the first instruction after `mov cr0` fetches from nowhere.
check "load is identity: PhysAddr equals VirtAddr" \
    "$(yes_if "$((${virt:-0}))" "$((${phys:-1}))")" \
    "VirtAddr ${virt:-?} vs PhysAddr ${phys:-?}"

# ---------------------------------------------------------------------
# 4. A loader can find and accept the multiboot header
# ---------------------------------------------------------------------
#
# `od -t x4` reports 4-byte words, so any offset it reports is 4-byte
# aligned by construction -- which is the other half of what multiboot1
# requires and the reason there is no separate alignment assertion here.
# Asserting `offset % 4 == 0` on a value that cannot be anything else
# would look like a check and test nothing.
header_offset=$(od -A d -t x4 -v -N 8192 "$IMAGE" \
    | awk '{ for (i = 2; i <= NF; i++) if ($i == "1badb002") { print $1 + (i - 2) * 4; exit } }')

check "the multiboot header is inside the first 8 KiB of the file" \
    "$(yes_if "$([ -n "$header_offset" ] && [ "$header_offset" -lt 8192 ] && echo y || echo n)" "y")" \
    "magic 0x1BADB002 not found in the first 8192 bytes; KEEP() in the linker script may have been dropped"

# The one condition a loader actually evaluates: magic + flags + checksum
# must be zero modulo 2^32. boot.s has the assembler compute the checksum,
# so this verifies the link preserved all three words in order rather than
# re-deriving arithmetic the assembler already did.
if [ -n "$header_offset" ]; then
    header_sum=$(od -A n -t u4 -v -N 12 -j "$header_offset" "$IMAGE" \
        | awk '{ for (i = 1; i <= NF; i++) s += $i } END { printf "%d", s % 4294967296 }')
else
    header_sum="no header"
fi
check "the multiboot header checksums to zero" \
    "$(yes_if "$header_sum" "0")" \
    "magic + flags + checksum was $header_sum, must be 0 mod 2^32"

# ---------------------------------------------------------------------
# 5. Nothing is left for a dynamic loader to do
# ---------------------------------------------------------------------

check "there is no dynamic section" \
    "$(yes_if "$(readelf -dW "$IMAGE" 2>/dev/null | grep -c 'no dynamic section')" "1")" \
    "$(readelf -dW "$IMAGE" 2>&1 | head -3)"

check "there are no relocations left in the image" \
    "$(yes_if "$(readelf -rW "$IMAGE" 2>/dev/null | grep -c 'no relocations')" "1")" \
    "$(readelf -rW "$IMAGE" 2>&1 | head -3)"

# ---------------------------------------------------------------------
# 6. The memory the entry stub assumes exists is actually reserved
# ---------------------------------------------------------------------

bss_start=$(sym __bss_start)
bss_end=$(sym __bss_end)
if [ -n "$bss_start" ] && [ -n "$bss_end" ]; then
    bss_size=$(( $((0x0$bss_end)) - $((0x0$bss_start)) ))
else
    bss_size=-1
fi

# The entry stub zeroes __bss_start..__bss_end with `rep stosb`. If the
# linker script stopped exporting either symbol the link would fail, but
# if the range ever came out *smaller* than what boot.s reserves inside
# it, the clear would silently stop partway and leave page-table entries
# holding whatever the firmware left in RAM. 3 x 4 KiB of page tables plus
# 64 KiB of stack is the floor.
check "the bss range spans at least the tables and the stack" \
    "$(yes_if "$([ "$bss_size" -ge 77824 ] && echo y || echo n)" "y")" \
    "__bss_start..__bss_end was $bss_size bytes, expected >= 77824"

# Reserved but not stored: bss must arrive as MemSiz beyond FileSiz, or the
# image would carry 76 KiB of zeroes on disk and, worse, a loader would
# have no instruction to reserve the range at all.
reserved=0
for pair in $(readelf -lW "$IMAGE" | awk '$1 == "LOAD" { print $5 ":" $6 }'); do
    reserved=$(( reserved + $(( ${pair##*:} )) - $(( ${pair%%:*} )) ))
done
check "the LOAD segments reserve zero-fill beyond what they store" \
    "$(yes_if "$([ "$reserved" -ge 77824 ] && echo y || echo n)" "y")" \
    "MemSiz exceeded FileSiz by $reserved bytes, expected >= 77824"

# The `:-1` fallback is load-bearing. An absent symbol has to make this
# check fail, and the obvious `:-0` does the opposite: 0 is 4 KiB aligned,
# so a stripped image would report a passing alignment assertion about a
# symbol that is not there. Found by stripping the image and watching this
# check stay green while five others went red.
pml4=$(sym boot_pml4)
check "boot_pml4 is 4 KiB aligned" \
    "$(yes_if "$(( $((0x0${pml4:-1})) % 4096 ))" "0")" \
    "boot_pml4 at 0x${pml4:-<missing>}; CR3 requires 4 KiB alignment"

stack_top=$(sym boot_stack_top)
check "boot_stack_top is 16-byte aligned" \
    "$(yes_if "$(( $((0x0${stack_top:-1})) % 16 ))" "0")" \
    "boot_stack_top at 0x${stack_top:-<missing>}; SysV requires rsp 16-byte aligned at a call"

echo
if [ "$FAILED" -eq 0 ]; then
    printf 'image shape: %d/%d checks passed\n' "$PASSED" "$((PASSED + FAILED))"
    exit 0
fi
printf 'image shape: %d/%d checks passed, %d FAILED\n' "$PASSED" "$((PASSED + FAILED))" "$FAILED"
exit 1



