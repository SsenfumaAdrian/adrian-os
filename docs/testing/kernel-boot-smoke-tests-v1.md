# ADRIAN OS Kernel Boot Smoke Tests v1

## Purpose
Validate the earliest kernel bring-up path.

## Smoke Checks
- workspace builds successfully
- kernel entry path is reachable
- init ordering compiles cleanly
- panic path is deterministic
- x86_64 arch module links cleanly
- memory/init/security/ipc/sched stubs integrate without dependency issues

## Future Emulator Checks
- boot output appears in order
- panic path observable
- init sequence visible
- kernel halts cleanly on fatal failure
