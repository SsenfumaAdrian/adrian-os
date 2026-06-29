# ADRIAN OS Boot Image Responsibility Model v1

## Purpose
Clarify what belongs inside the boot-image crate versus the Axiom kernel crate.

## Axiom Kernel Owns
- kernel subsystem logic
- process/thread/runtime model
- early init sequencing
- internal kernel entry boundary
- MM/security/scheduler/IPC structure

## Boot Image Owns
- external wrapper concerns
- future entry symbol exposure
- future linker/build target specialization
- boot artifact packaging path
- experiment-oriented emulator integration

## Rule
The boot-image crate must remain thin and purpose-specific.
