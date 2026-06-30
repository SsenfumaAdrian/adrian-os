# ADRIAN OS Wrapper-Kernel Ownership Boundary v1

## Rule 1
Wrapper-side code must not implement kernel initialization logic.

## Rule 2
Kernel core must not absorb temporary wrapper experiment semantics as hidden truth.

## Rule 3
Synthetic handoff remains wrapper-side and temporary until replaced or mapped into production-consistent semantics.

## Rule 4
Axiom becomes authoritative immediately after the transition boundary is crossed.
