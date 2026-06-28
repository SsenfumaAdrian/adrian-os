# ADRIAN OS UART Serial Enablement Tests v1

## Immediate Validation
- cargo check passes
- UART register constants compile
- serial init path compiles
- transmit byte path compiles

## Future Emulator Validation
- serial init is invoked during arch bring-up
- debug markers become visible in QEMU
- panic marker visible on panic path
