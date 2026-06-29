# ADRIAN OS Wrapper Stage Integration Boundary v1

## Wrapper Owns
- entry stage ordering
- bridge stage ordering
- invocation stage ordering
- synthetic experiment-side coordination

## Axiom Owns
- post-invocation initialization
- marker path
- halt behavior

## Rule
Do not blur wrapper-side coordination with kernel-side ownership.
