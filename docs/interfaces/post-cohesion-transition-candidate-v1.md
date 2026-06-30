# Post-Cohesion Transition Candidate v1

## Purpose
Promote the MRT-1 transition candidate from a flat status string into a
small **structured wrapper-side type**, now that wrapper-side FMH-1
summary cohesion is complete (bridge + invoke + transition).

## New type: TransitionCandidate
Fields (all placeholder-grade, no production semantics):

| Field               | Meaning                                                   |
|---------------------|-----------------------------------------------------------|
| version             | Candidate model version                                   |
| label               | Stable identifier ("mrt-1")                               |
| flow_wired          | Wrapper-side flow conceptually wired                      |
| summary_cohesion    | FMH-1 summary cohesion complete                           |
| real_axiom_call     | MUST stay false until a real-crossing pack flips it       |
| real_boot_context   | MUST stay false until a real-BootContext pack flips it    |
| phase_label         | Human-readable readiness phase                            |

### Methods
- `mrt1_current()` -> current candidate (const)
- `ready_for_real_crossing_gate()` -> flow_wired && summary_cohesion
- `is_experiment_only()` -> !real_axiom_call && !real_boot_context

## Safety invariants
- `real_axiom_call` and `real_boot_context` remain **false** in this pack.
- The "crossing gate" may report open, but NO crossing is performed here.
- A future, explicit pack must perform any real crossing intentionally.

## Guarantees
- No behavior change (placeholder semantics only)
- No real Axiom call
- No real BootContext construction
- Compile-clean (`cargo check` expected to pass)

## Status
- Phase: MRT-1 active, post-cohesion, experiment-only
- Crossing gate: open (prerequisites met) but uncrossed
- Next: design the first real (still experiment-bounded) crossing pack
  that constructs a synthetic BootContext on the wrapper side
