# ADRIAN OS Handoff-to-Invocation Alignment v1

## Purpose
Document how the wrapper-side invocation stage conceptually aligns with the synthetic handoff model.

## Current Meaning
- synthetic handoff represents temporary wrapper-owned experiment state
- invocation represents the future consumer of that prepared handoff
- both remain compile-clean and conceptual for FBE-1

## Rule
This alignment is a staged experiment-side model and must not be mistaken for the final production Axiom invocation path.
