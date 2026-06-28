# ADRIAN OS Bootable Kernel Plan v1

## Purpose
Define the path from a cargo-check-valid kernel crate to a bootable kernel artifact.

## Stages
1. coherent no_std kernel library
2. explicit kernel entry model
3. architecture-aware entry handoff
4. binary artifact strategy
5. linker/target planning
6. bootloader/Halo integration
7. emulator boot path

## Near-Term Goals
- define kernel entry function boundaries
- separate boot handoff logic from generic init flow
- prepare for linker-script and target work
- document QEMU-oriented boot strategy

## Future Goals
- dedicated bootable kernel binary crate or target
- linker script
- target JSON if required
- serial output
- boot image packaging
