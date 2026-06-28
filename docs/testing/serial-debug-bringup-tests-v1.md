# ADRIAN OS Serial Debug Bring-Up Tests v1

## Immediate Validation
- cargo check passes
- debug module links cleanly
- serial backend module compiles
- x86_64 serial structures compile

## Future Emulator Validation
- serial init path callable
- fixed markers emitted in expected order
- panic marker visible
- output captured in QEMU console/log
