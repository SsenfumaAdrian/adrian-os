# Rian Boot Image Linker Area

## Current State

Empty, and now superseded. The real linker script lives at
[`rian/bare-metal/linker/rian.ld`](../../bare-metal/linker/rian.ld).

This directory was reserved before there was a bare-metal artifact to link,
on the assumption that the boot-image path would grow one. It did not, and
it should not: `rian/boot-image` turned out to be the *hosted* dev loop — a
normal `std` binary that calls `kernel_entry` from a Linux process so the
init sequence can be exercised with no hardware. A hosted binary is linked
by the host toolchain and has no layout to describe.

The artifact that does need a layout is `rian/bare-metal`, which is a
separate crate for reasons its README explains (it is `#![no_main]`, links
only for `x86_64-unknown-none`, and must not share a cargo invocation with
anything that enables the kernel's `std` feature). Its linker script is
kept beside it rather than here, so that the crate, its target
configuration and its memory layout are one unit.

Left in place rather than deleted because the layout it described —
`linker/` beside the crate that uses it — is the layout the bare-metal
crate adopted, and this file is where that convention is recorded.

## Ownership

Nothing. See `rian/bare-metal/linker/`.
