# `rian/bare-metal` — the image a bootloader loads

This crate is the ELF that runs on the machine. Everything else in the
repository is either kernel code that this links, or a hosted harness that
exercises kernel code without a machine.

It exists because of a gap in what the project could verify. Before it,
`rian/kernel` compiled and its tests passed, and `rian/boot-image` ran the
init sequence in a Linux process — but nothing had ever established that
the kernel could run with no operating system underneath it. Every module
under `arch::x86_64` was unverifiable in principle: hosted port I/O is
stubbed, so a `serial` backend that writes to the wrong port, or a boot
path that assumes a stack it does not have, would pass every test in the
repository. This crate closes that gap, and `boot tested (emulated)`
becomes a verification level the project can actually reach.

## What is here

| File | What it is |
| --- | --- |
| `src/boot.s` | The entry stub: multiboot1 header, bss clear, long-mode bring-up, GDT, the call into Rust. |
| `src/main.rs` | `rian_main` — translates the bootloader handoff into a `BootContext` and calls `kernel_entry_and_halt`. |
| `linker/rian.ld` | Layout: header first, image at 1 MiB, `__bss_start`/`__bss_end` exported. |
| `.cargo/config.toml` | Target, linker script, code model, relocation model. |
| `Cargo.toml` | Its own workspace root. See below. |

Verification lives outside the crate, in `tools/image/`:
`verify_shape.sh` asserts sixteen properties of the linked ELF, and
`verify_boot.sh` asserts twelve properties of the serial log a boot
produces. Both are POSIX `sh` over `readelf`, `od` and `awk`, so they run
in any CI image that can build Rust.

## Building it

```sh
rustup target add x86_64-unknown-none
cd rian/bare-metal
cargo build --release
cd ../..
sh tools/image/verify_shape.sh rian/bare-metal/target/x86_64-unknown-none/release/rian
```

The `cd` is not optional. Cargo discovers `.cargo/config.toml` by walking
up from the working directory, and that file is where the target, the
linker script, `-C code-model=small` and `-C relocation-model=static` are
set. Building from the repository root finds none of them, and the four
together are the difference between a loadable image and a
position-independent one that resets the machine with no output.

Booting it:

```sh
qemu-system-x86_64 -kernel rian/bare-metal/target/x86_64-unknown-none/release/rian \
    -m 128M -display none -no-reboot -serial file:serial.log
sh tools/image/verify_boot.sh serial.log
```

QEMU will not exit on its own. The image ends in `halt_forever()`, and it
contains no `isa-debug-exit` write, because a port write whose only
purpose is to tell a test harness the answer is test-only code inside the
artifact that ships. Interrupt it, or wrap it in `timeout` as CI does.

A healthy log reads:

```
RIAN: multiboot1 handoff
entry
boot-context
arch
memory
security
ipc
scheduler
process
thread
idle
RIAN: INIT COMPLETE, HALTING
```

## Why it is outside the workspace

`Cargo.toml` carries an empty `[workspace]` table and the root
`Cargo.toml` lists this directory under `exclude`. Both are needed, and
neither is tidiness:

1. This crate is `#![no_main]` and links only for `x86_64-unknown-none`.
   As a workspace member it would be part of a plain `cargo build` at the
   repository root, which builds for the host and fails on a missing
   `main`.
2. Under resolver 2, features unify across everything selected in a single
   invocation. `rian/boot-image` depends on `adrian-kernel` with
   `features = ["std"]`. Sharing an invocation would hand this image a
   `std` kernel — silently, and it would still link. That is the one
   configuration a bare-metal image must never have.

The cost is that `cargo build` at the root does not build this crate, so
CI has a separate job for it (`image-build`). That is a real cost and it
is the reason the job exists.

## Design decisions worth knowing before changing anything

**`x86_64-unknown-none`, not a custom target spec.** It is stable and
tier 2, `core` and `compiler_builtins` ship prebuilt via `rustup target
add`, and it links with `rust-lld` — no external linker. The last point
decides it: the development host cannot run the GNU toolchain (Smart App
Control blocks it) and MSVC cannot emit ELF. Its defaults assume a
higher-half relocatable kernel, which is why `code-model` and
`relocation-model` are overridden.

**Multiboot1, not 2.** `qemu -kernel` loads a multiboot1 image directly.
Multiboot2 needs GRUB and an ISO built with `xorriso`, which would put two
more tools between a commit and the first boot test.

**Assembly through `global_asm!(include_str!(...))`.** Goes to LLVM's
integrated assembler, so no external assembler, no binutils, no `cc`
crate. `boot.s` therefore holds one non-obvious invariant: it contains no
`{` or `}`, because `global_asm!` parses braces as format arguments. The
alternative is `options(raw)` at the call site; keeping braces out is the
cheaper thing to hold.

**Identity-mapped, not higher-half.** A higher-half kernel needs an
address-space split and a matching code model — roadmap step 5. Doing it
here would make the first bare-metal boot depend on two unverified things
at once instead of one.

**The bss is cleared by `_start`, not by the loader.** QEMU and GRUB both
zero-fill the gap between `FileSiz` and `MemSiz`. Rust's zero-initialized
statics and the three boot page tables are not things to rest on somebody
else's implementation of that.

## What this image does not do yet

Each of these is a roadmap step, and none of them is stubbed with a
plausible-looking value — `BootContext::empty()` leaves them empty and
`main.rs` says why for each field.

- **No IDT, no TSS, no exception handlers.** Any fault from `rian_main`
  onward is a triple fault: the machine resets and the serial log stops
  mid-sentence. Step 3, and the next thing that should exist.
- **The multiboot memory map is not parsed.** `entry_count` stays 0 and
  `early_mm_init` is still seeded with nothing. Step 4.
- **No framebuffer.** The header requests no video mode, so the loader
  provides none.
- **No tests in this crate.** `cargo test` would build it for the host,
  and it does not link for the host. `handoff_label` is split out as a
  pure function so that a future harness has something to assert on;
  everything else here can only be verified by booting, which is what
  `tools/image/` is for.
