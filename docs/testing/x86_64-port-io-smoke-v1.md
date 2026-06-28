# ADRIAN OS x86_64 Port I/O Smoke Validation v1

## Immediate Validation
- cargo check passes
- unsafe port I/O module compiles
- serial backend still compiles cleanly

## Future Runtime Validation
- serial init path executes
- fixed debug markers become visible in QEMU
- panic marker becomes visible
- no unexpected trap during early serial path
