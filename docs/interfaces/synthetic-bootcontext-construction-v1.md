# Synthetic BootContext Construction v1

## Purpose
Construct the first REAL artifact-shaped object on the wrapper side: a
synthetic `SyntheticBootContext` whose field shape mirrors the expected
Axiom `BootContext` contract. This is the concrete precursor to the first
genuine wrapper -> Axiom boundary crossing.

## Field shape (Minimal v1)
| Field               | Type           | Meaning                                  |
|---------------------|----------------|------------------------------------------|
| version             | u32            | BootContext contract version             |
| arch_label          | &'static str   | Target architecture label ("x86_64")     |
| memory_map_present  | bool           | Memory map (synthetically) present       |
| serial_available    | bool           | Serial backend (synthetically) available |
| experiment_mode     | bool           | MUST be true while synthetic-only        |

## Placement
`axiom/boot-image/src/bridge.rs` â€” the wrapper-side BootContext bridge.

## Methods
- `SyntheticBootContext::fbe1_default()` -> const default for FBE-1
- `is_well_formed()` -> structural validity of the synthetic precursor

## Safety invariants
- This is SYNTHETIC. It is not firmware-provided and is not the real
  Axiom BootContext.
- NO Axiom call is performed.
- The candidate's `real_boot_context` flag stays FALSE: this synthetic
  construction does not count as a real BootContext.
- `experiment_mode` stays true.

## Contract consistency note
The Minimal v1 shape is the agreed contract between the wrapper and
Axiom. When Axiom's real `BootContext` is finalized, its first five
fields should match this shape (or this shape should be updated in
lockstep) so the future real crossing has a stable, shared layout.

## Guarantees
- Compile-clean (`cargo check` expected to pass)
- No behavior change beyond constructing/validating a synthetic object

## Status
- Phase: MRT-1 post-cohesion, synthetic precursor constructed
- Crossing gate: open but uncrossed
- Next: design the first real (still experiment-bounded) crossing that
  passes this synthetic context across the boundary toward Axiom
