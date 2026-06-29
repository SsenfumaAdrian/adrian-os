# ADRIAN OS Boot Artifact Entry Strategy v1

## Purpose
Define how the future boot artifact should expose and manage the entry path into Axiom.

## Core Principle
External entry concerns belong to the boot-image path, while kernel-core init remains inside Axiom.

## Boot Image Responsibilities
- future external entry symbol ownership
- artifact-facing entry mechanics
- boot-environment adaptation
- handoff into kernel core entry boundary

## Kernel Responsibilities
- BootContext validation
- internal initialization flow
- early subsystem progression
- panic and debug behavior

## Rule
Do not push boot-artifact entry ownership into the kernel core crate.
