# ADRIAN OS Boot Image to Axiom Invocation Plan v1

## Purpose
Define how the boot-image path should eventually invoke Axiom kernel entry.

## Core Principle
The invocation step belongs to the boot-image wrapper side, not the kernel core.

## Wrapper Responsibilities
- own wrapper entry
- own BootContext bridge preparation
- own invocation into Axiom entry boundary
- document temporary experiment assumptions

## Kernel Responsibilities
- validate handoff
- own all internal early initialization after invocation
