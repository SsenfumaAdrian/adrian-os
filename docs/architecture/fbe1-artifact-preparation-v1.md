# ADRIAN OS FBE-1 Artifact Preparation v1

## Purpose
Define the artifact expectations for the first runnable boot experiment.

## Minimal Artifact Goals
- wrapper-side entry path
- bridge into BootContext-compatible state
- path into Axiom internal entry
- serial-first observability
- deterministic halt behavior

## Rule
The FBE-1 artifact should be the smallest artifact that can prove runtime entry and visible serial output.
