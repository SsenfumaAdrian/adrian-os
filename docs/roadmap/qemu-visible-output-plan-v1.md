# ADRIAN OS QEMU Visible Output Plan v1

## Purpose
Define the path toward visible early boot markers through the serial backend in QEMU.

## Immediate Goals
- initialize serial early
- emit fixed milestone markers
- ensure line-based output is terminal friendly
- make panic output visible through same path

## Expected Marker Sequence
- AXIOM: ENTRY
- AXIOM: BOOT CONTEXT OK
- AXIOM: ARCH INIT
- AXIOM: MM INIT
- AXIOM: SECURITY INIT
- AXIOM: IPC INIT
- AXIOM: SCHED INIT
- AXIOM: HALT

## Future Goal
Observe this marker flow in QEMU serial output.
