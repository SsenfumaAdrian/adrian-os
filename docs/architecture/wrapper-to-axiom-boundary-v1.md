# ADRIAN OS Wrapper-to-Axiom Boundary v1

## Purpose
Define the conceptual boundary between wrapper-side FBE-1 experiment flow and Axiom-owned initialization flow.

## Boundary Concept
entry -> bridge -> synthetic handoff -> invoke || Axiom kernel_entry(&BootContext) -> init -> markers -> halt

## Wrapper Owns
- entry staging
- bridge staging
- synthetic handoff staging
- invocation preparation

## Axiom Owns
- BootContext validation
- internal initialization
- marker emission
- deterministic halt behavior
