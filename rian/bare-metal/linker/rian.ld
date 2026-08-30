/*
 * Memory layout for the Rian bare-metal image.
 *
 * Consumed by rust-lld via `-C link-arg=-Tlinker/rian.ld` in
 * ../.cargo/config.toml. The three properties that matter, in the order a
 * bootloader cares about them:
 *
 *   1. The multiboot header comes first, so it lands well inside the
 *      first 8 KiB of the file where the loader scans for it.
 *   2. The image loads at 1 MiB. Below that is real-mode IVT, BIOS data,
 *      VGA memory and option ROMs; 1 MiB is the lowest address that is
 *      conventionally free and it is what every multiboot loader expects.
 *   3. `__bss_start` / `__bss_end` are exported so the entry stub can
 *      zero the bss itself instead of trusting the loader to. QEMU's ELF
 *      loader does zero-fill, GRUB does too, but Rust statics assume zero
 *      and the page tables built in bss must be zero -- so this is not a
 *      property to inherit from somebody else's implementation.
 */

ENTRY(_start)

/*
 * 1 MiB. Not a tunable: changing it means changing what the bootloader is
 * asked to do, and `tools/image/verify_shape.sh` asserts the linked
 * result matches.
 */
IMAGE_LOAD_ADDRESS = 0x100000;

SECTIONS
{
    . = IMAGE_LOAD_ADDRESS;

    /*
     * Header and 32-bit entry stub together, and first. KEEP() because
     * nothing in the program references the header -- only the bootloader
     * does, by scanning -- so any section-garbage-collection pass would
     * be entirely correct to discard it, and the resulting image would
     * fail to boot with no diagnostic.
     */
    .boot : ALIGN(8)
    {
        KEEP(*(.multiboot_header))
        KEEP(*(.boot.text))
    }

    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
        /*
         * Linked `-C relocation-model=static`, so these should be empty.
         * Placed rather than discarded so that if they ever stop being
         * empty the image still links and the shape check reports it,
         * instead of the linker silently dropping relocation state the
         * program needs.
         */
        *(.got .got.plt)
    }

    /*
     * No `(NOLOAD)` attribute. The input sections are already SHT_NOBITS,
     * so the output section is too, and the enclosing segment gets
     * MemSiz > FileSiz -- which is precisely the instruction that tells a
     * loader to reserve and zero this range. Spelling `(NOLOAD)` risks
     * the opposite reading, and the reserve-and-zero behavior is what the
     * 64 KiB stack and the boot page tables depend on.
     */
    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    __image_end = .;

    /*
     * There is no unwinder (panic = abort) and no runtime to read notes,
     * so these are dead weight in an image that a bootloader copies byte
     * for byte into low memory.
     */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.gcc_except_table)
        *(.comment)
        *(.note .note.*)
    }
}
