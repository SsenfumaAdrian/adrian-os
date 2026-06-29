# ADRIAN OS First Boot Success Criteria v1

## BE-S1
- artifact loads in QEMU experiment path
- Axiom entry reached
- at least one visible serial marker appears
- deterministic halt occurs

## BE-S2
Visible milestone sequence:
- AXIOM: ENTRY
- AXIOM: BOOT CONTEXT OK
- AXIOM: ARCH INIT
- AXIOM: MM INIT
- AXIOM: SECURITY INIT
- AXIOM: IPC INIT
- AXIOM: SCHED INIT
- AXIOM: HALT

## BE-S3
- panic marker visible on forced failure path
