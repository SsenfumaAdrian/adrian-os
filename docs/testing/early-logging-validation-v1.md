# ADRIAN OS Early Logging Validation v1

## Validation Goals
- debug module compiles
- early init can call debug marker functions
- panic path can call panic marker before halting
- cargo check remains clean

## Future Emulator Validation
- ordered boot markers appear
- panic marker appears on failure
- serial capture is reproducible in QEMU
