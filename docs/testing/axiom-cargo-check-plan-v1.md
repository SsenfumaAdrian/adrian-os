# ADRIAN OS Axiom Cargo Check Plan v1

## Purpose
Prepare the Axiom kernel crate for early cargo check discipline.

## Immediate Goals
- ensure coherent Rust module graph
- ensure no_std-compatible panic handling
- keep dependencies empty unless justified
- avoid accidental std usage
- keep placeholder APIs simple and explicit

## First Validation Targets
1. workspace structure is valid
2. adrian-kernel crate resolves all modules
3. panic handler exists
4. early init path compiles cleanly
5. no accidental duplicate symbols or invalid module paths

## Next Follow-Up
- run cargo check
- resolve compiler diagnostics
- add formatting/lint workflow
- begin target/build planning for kernel image flow
