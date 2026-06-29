# ADRIAN OS Boot Image Linker Strategy v1

## Purpose
Define how linker concerns should be introduced into ADRIAN OS boot-artifact evolution.

## Core Principle
Linker concerns belong closer to the boot-image artifact path than to the kernel core crate.

## Ownership
### Kernel Core
- internal logic only
- not final boot artifact placement policy

### Boot Image
- linker integration ownership
- artifact layout ownership
- external entry symbol ownership
- boot-target coupling

## Staging
- documentation first
- placeholder structure second
- experimental linker script later
- real boot-target integration after target workflow is ready
