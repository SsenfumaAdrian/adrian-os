# ADRIAN OS Early Debug Interface v1

## Purpose
Define the earliest kernel-visible debug interface for bring-up.

## Requirements
- no_std safe
- no allocator dependency
- fixed-string friendly
- usable during early init
- usable during panic path where possible

## Initial API Direction
- debug_marker(&str)
- panic_marker(&str)

## Backend Direction
Initial backend may be a no-op placeholder.
Later backend should target serial-first output for emulator and real hardware bring-up.
