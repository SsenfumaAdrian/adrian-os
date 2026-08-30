# Rian bare-metal entry stub.
#
# What a bootloader hands us and what Rust needs are two different
# machines. This file is the distance between them: a multiboot1 header so
# a loader will accept the image at all, then 32-bit protected-mode code
# that clears the bss, proves the CPU has a 64-bit mode, builds an
# identity map, switches on long mode, and only then calls into Rust.
#
# Assembled by LLVM's integrated assembler through `global_asm!` in
# main.rs rather than by an external `as`. That is not a stylistic
# preference: the development host cannot run the GNU toolchain (Smart App
# Control blocks it), so a build step needing binutils would be a build
# step that only ever runs in CI.
#
# Conventions in force here, both deliberate:
#   * Intel syntax -- `global_asm!` defaults to it on x86 and the manuals
#     this code is checked against are written in it.
#   * No `{` or `}` anywhere in this file. `global_asm!` parses braces as
#     format arguments, and the alternative is `options(raw)` at the call
#     site. Keeping braces out is the cheaper invariant to hold.

# ---------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------

# Multiboot1, not 2. QEMU's `-kernel` loads a multiboot1 image directly;
# multiboot2 needs GRUB and an ISO built with xorriso, which would put a
# bootable artifact behind two more tools before it could be tested once.
.set MULTIBOOT1_HEADER_MAGIC, 0x1BADB002

# Bit 0: page-align loaded modules. Bit 1: supply the memory map. The
# second is requested now even though nothing reads it yet -- roadmap
# step 4 is exactly "the allocator is seeded with something", and it needs
# the loader to have been asked for a map before it can find one.
.set MULTIBOOT1_HEADER_FLAGS, 0x00000003

.set GDT_CODE_SELECTOR, 0x08
.set GDT_DATA_SELECTOR, 0x10

# Single-character reasons written to COM1 by `boot_fail`.
.set FAIL_NO_CPUID_LEAF, 0x43   # 'C'
.set FAIL_NO_LONG_MODE,  0x4C   # 'L'

.set COM1_DATA_PORT, 0x3F8

# ---------------------------------------------------------------------
# Multiboot1 header
# ---------------------------------------------------------------------
#
# Placed in its own section so linker/rian.ld can put it first and KEEP()
# it. The checksum is computed by the assembler instead of being written
# out as a literal: it has to satisfy magic + flags + checksum == 0 mod
# 2^32, and a hand-computed constant here would be a silent boot failure
# the first time the flags change.

.section .multiboot_header, "a"
.align 8
multiboot_header:
    .long MULTIBOOT1_HEADER_MAGIC
    .long MULTIBOOT1_HEADER_FLAGS
    .long -(MULTIBOOT1_HEADER_MAGIC + MULTIBOOT1_HEADER_FLAGS)
multiboot_header_end:

# ---------------------------------------------------------------------
# 32-bit entry
# ---------------------------------------------------------------------
#
# Entered with, per the multiboot1 specification: protected mode on,
# paging off, interrupts disabled, A20 enabled, flat 4 GiB segments in
# every selector, eax = 0x2BADB002, ebx = physical address of the
# multiboot information structure, and esp undefined -- which is why the
# first job here is to own a stack.

.section .boot.text, "ax"
.code32
.global _start
_start:
    cli
    cld

    # The handoff arrives in eax and ebx. `rep stosb` below clobbers eax,
    # ecx and edi, so move the magic into esi, which it does not touch;
    # ebx survives untouched.
    mov esi, eax

    # Zero the bss before anything can read it. Three separate things
    # depend on this and all three fail silently without it: every
    # zero-initialized Rust static (the spin locks, the global boot trace,
    # the serial timeout counter), the page tables built below -- whose
    # unused entries must not be interpreted as present mappings -- and
    # the handoff slots at the bottom of this file.
    #
    # Done here rather than trusted to the bootloader. QEMU and GRUB both
    # zero-fill the gap between FileSiz and MemSiz, but "the loader
    # probably does it" is not the kind of thing the first three page
    # tables in the system should rest on.
    #
    # Runs with no stack, which is fine: it touches no memory but its
    # target, and nothing may be pushed before the stack exists.
    lea edi, [__bss_start]
    lea ecx, [__bss_end]
    sub ecx, edi
    xor eax, eax
    rep stosb

    # The stack lives inside the bss that was just cleared, so it has to
    # be claimed after the clear, never before.
    lea esp, [boot_stack_top]

    # Park the handoff where the 64-bit half can pick it up. Both stores
    # are 32-bit into 8-byte slots; the upper halves stay zero from the
    # clear above, so the 64-bit reads zero-extend correctly.
    mov [multiboot_magic], esi
    mov [multiboot_info_ptr], ebx

    # Does this CPU have a 64-bit mode? Checked rather than assumed
    # because the alternative failure is the worst one available: a far
    # return into a 64-bit code segment on a CPU without long mode
    # triple-faults, and a triple fault resets the machine with nothing
    # printed and nothing logged.
    #
    # Deliberately no check for CPUID itself -- the EFLAGS.ID toggle
    # dance. Reaching this code requires a multiboot loader, and every CPU
    # capable of running one postdates CPUID by years. The extended-leaf
    # check below is the one that earns its keep, since plenty of 32-bit
    # CPUs have CPUID and no long mode.
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb fail_no_cpuid_leaf

    mov eax, 0x80000001
    cpuid
    test edx, 0x20000000        # CPUID.80000001h:EDX.LM, bit 29
    jz fail_no_long_mode

    # Identity-map the first 1 GiB: PML4[0] -> PDPT[0] -> PD[0..511], with
    # 2 MiB pages. One gigabyte because the image, its stack, and anything
    # a bootloader placed near them are all far below that mark. 2 MiB
    # pages because they keep the entire map inside three 4 KiB tables,
    # and three fixed tables need no allocator -- which matters, because
    # there is not one yet: `early_mm_init(&[])` is still seeded with
    # nothing.
    #
    # Identity rather than higher-half on purpose. A higher-half kernel
    # needs an address-space split and a matching code model, which is
    # roadmap step 5; doing it here would make the first bare-metal boot
    # depend on two unverified things at once instead of one.
    lea edi, [boot_pd]
    mov eax, 0x00000083         # frame 0, HUGE | WRITABLE | PRESENT
    mov ecx, 512
fill_page_directory:
    # Only the low dword is written. The high dword of every entry stays
    # zero from the bss clear, which is what "no NX bit, no address bits
    # above 32" means for an identity map of low memory.
    mov [edi], eax
    add eax, 0x200000           # next 2 MiB frame
    add edi, 8
    loop fill_page_directory

    # Upper levels: one entry each, pointing at the level below.
    lea eax, [boot_pd]
    or eax, 0x03                # WRITABLE | PRESENT
    mov [boot_pdpt], eax

    lea eax, [boot_pdpt]
    or eax, 0x03
    mov [boot_pml4], eax

    lea eax, [boot_pml4]
    mov cr3, eax

    # PAE first. Long mode is PAE paging; setting EFER.LME without it is
    # accepted quietly and then faults when paging is switched on, which
    # puts the diagnostic one instruction away from where the mistake is.
    mov eax, cr4
    or eax, 0x20                # CR4.PAE, bit 5
    mov cr4, eax

    # EFER.LME, MSR 0xC0000080.
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100               # EFER.LME, bit 8
    wrmsr

    # This is the instruction that activates long mode: protected mode is
    # already on, so enabling paging with PAE and LME set puts the CPU in
    # 64-bit mode -- running in 32-bit compatibility mode until a 64-bit
    # code segment is loaded, which is the next two steps.
    mov eax, cr0
    or eax, 0x80000000          # CR0.PG, bit 31
    mov cr0, eax

    # Whatever GDT the bootloader left behind is a 32-bit one, so it has
    # no descriptor with the L bit set and cannot be used to leave
    # compatibility mode. Ours can.
    lgdt [boot_gdt_pointer]

    # A far return rather than a far jump. `retf` has one unambiguous
    # spelling under LLVM's integrated assembler, whereas the immediate
    # seg:offset form of a far `jmp` does not, and the integrated
    # assembler is the only one this project can rely on being present.
    # `retf` pops the offset first and the selector second, so the
    # selector is pushed first.
    lea eax, [long_mode_entry]
    push GDT_CODE_SELECTOR
    push eax
    retf

# ---------------------------------------------------------------------
# 32-bit failure paths
# ---------------------------------------------------------------------

fail_no_cpuid_leaf:
    mov al, FAIL_NO_CPUID_LEAF
    jmp boot_fail

fail_no_long_mode:
    mov al, FAIL_NO_LONG_MODE
    jmp boot_fail

# One character on COM1, then stop. `al` holds the reason:
#
#   'C' -- CPUID has no 0x80000001 leaf
#   'L' -- CPUID reports no long mode
#
# Raw port writes with no UART programming and no string loop, because
# every dependency this could take is unavailable or untrustworthy at this
# point: there is no Rust yet, so `debug::serial` is out of reach, and the
# machine has just been shown to be one the kernel cannot run on. Under
# QEMU the default 8250 state transmits well enough for the byte to reach
# `-serial`; on hardware with no UART at 0x3F8 it goes nowhere, which is
# still strictly better than resetting with no output at all.
#
# `hlt` in a loop rather than a bare loop, for the same reason
# `panic::halt_forever` uses it: a parked core stops burning power instead
# of spinning at 100% until someone notices.
boot_fail:
    mov dx, COM1_DATA_PORT
    out dx, al
    mov al, 0x0D                # CR
    out dx, al
    mov al, 0x0A                # LF
    out dx, al
boot_fail_halt:
    cli
    hlt
    jmp boot_fail_halt

# ---------------------------------------------------------------------
# 64-bit entry
# ---------------------------------------------------------------------

.code64
long_mode_entry:
    # The selectors left over from compatibility mode still index the
    # bootloader's GDT, which no longer exists as far as this kernel is
    # concerned. In 64-bit mode ds/es/fs/gs are ignored for addressing,
    # but a stale selector is a fault waiting for the first instruction
    # that loads from one, so they are all pointed at our flat data
    # descriptor.
    mov ax, GDT_DATA_SELECTOR
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    # Reload rsp with a 64-bit write. The 32-bit `lea esp` above left the
    # upper half zero, which happens to be correct, but a stack pointer
    # that is right by accident is a stack pointer that stops being right
    # the moment the image moves.
    lea rsp, [boot_stack_top]

    # SysV: first two integer arguments in rdi and rsi. Both slots were
    # written 32 bits wide and a 32-bit load zero-extends, which is the
    # right width -- a multiboot information pointer is a physical address
    # below 4 GiB by construction.
    mov edi, [multiboot_magic]
    mov esi, [multiboot_info_ptr]
    call rian_main

    # `rian_main` is `-> !`. Reaching here means it returned anyway, so
    # stop deliberately rather than executing whatever the linker happened
    # to place next.
long_mode_unreachable:
    cli
    hlt
    jmp long_mode_unreachable

# ---------------------------------------------------------------------
# Global descriptor table
# ---------------------------------------------------------------------
#
# The minimum that can leave compatibility mode: a null descriptor, one
# 64-bit code segment, one data segment. Flat and DPL 0 -- there is no
# ring 3 yet, and a real GDT with a TSS is roadmap step 3's business,
# alongside the IDT that would let a fault say something.
#
# Bit layout of each entry, high to low: base[31:24] | flags | limit[19:16]
# | access | base[23:16] | base[15:0] | limit[15:0].
#   code: access 0x9A = present, DPL 0, code, executable, readable
#         flags  0xA  = granularity 4 KiB, L set (64-bit), D clear
#   data: access 0x92 = present, DPL 0, data, writable
#         flags  0xC  = granularity 4 KiB, D/B set (32-bit operands)

.section .rodata
.align 8
boot_gdt:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF    # GDT_CODE_SELECTOR, 0x08
    .quad 0x00CF92000000FFFF    # GDT_DATA_SELECTOR, 0x10
boot_gdt_end:

# `lgdt` reads a 16-bit limit followed by the base. Loaded from 32-bit
# code, so the base is 32 bits wide here; the image is linked at 1 MiB so
# it fits, and the 32-bit absolute relocation this emits is only legal
# because the link is `-C relocation-model=static`.
boot_gdt_pointer:
    .word boot_gdt_end - boot_gdt - 1
    .long boot_gdt

# ---------------------------------------------------------------------
# Reserved memory
# ---------------------------------------------------------------------
#
# All of it in bss, so none of it costs a byte in the image the bootloader
# copies -- it becomes MemSiz beyond FileSiz, and the clear at the top of
# `_start` is what actually makes it zero.

.section .bss, "aw", @nobits

# CR3 requires a 4 KiB-aligned PML4, and each level below it is one page.
# `tools/image/verify_shape.sh` asserts this alignment survived the link,
# because a misaligned CR3 load is a #GP with no message attached.
.align 4096
boot_pml4:
    .skip 4096
boot_pdpt:
    .skip 4096
boot_pd:
    .skip 4096

# 64 KiB of stack. Generous for a boot path whose deepest call chain is
# `_start` -> `rian_main` -> `kernel_entry` -> init -> a subsystem, and
# cheap because bss costs image size nothing.
#
# `.align 16` and a size that is a multiple of 16 so that `boot_stack_top`
# satisfies the SysV requirement for rsp at a call boundary. The target
# spec disables SSE, so nothing here spills a 16-byte-aligned register
# today -- but ABI conformance that holds only because of a feature flag
# is not conformance.
.align 16
boot_stack_bottom:
    .skip 65536
boot_stack_top:

# Where the multiboot handoff waits out the long-mode transition. Eight
# bytes each rather than four so the 64-bit side can widen its reads
# later without moving anything.
.align 8
multiboot_magic:
    .skip 8
multiboot_info_ptr:
    .skip 8
