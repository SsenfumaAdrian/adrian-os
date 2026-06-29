# ADRIAN OS Boot Experiment Assumptions v1

## Purpose
Define what assumptions are acceptable in the earliest ADRIAN OS boot experiments.

## Core Principle
Temporary experiment shortcuts are allowed only when they are explicit, isolated, and documented.

## Allowed Early Assumptions
- simplified wrapper-side BootContext construction
- x86_64 + QEMU-only early bring-up focus
- serial-output-first validation
- early halt after milestone output
- experiment-only artifact path before full Halo integration

## Forbidden Drift
- changing kernel ownership boundaries casually
- hiding synthetic behavior inside production-facing semantics
- confusing experiment path with final trust-chain path
