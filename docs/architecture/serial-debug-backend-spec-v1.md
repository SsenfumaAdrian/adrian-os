# ADRIAN OS Serial Debug Backend Specification v1

## Purpose
Define the first real early-output backend for Axiom bring-up.

## Why Serial First
- emulator friendly
- simple hardware model
- low dependency
- panic-safe potential
- ideal for early x86_64 bring-up

## Initial Scope
- fixed-string output
- milestone markers
- panic marker support
- x86_64 first
- QEMU-focused first validation

## Backend Strategy
1. kernel-facing debug API stays generic
2. backend implementation lives under debug/serial
3. architecture/platform specifics stay isolated
4. later evolution may support framebuffer or ring-buffer logging
