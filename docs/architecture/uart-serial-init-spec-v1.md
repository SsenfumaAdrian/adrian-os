# ADRIAN OS UART Serial Initialization Specification v1

## Purpose
Define the early UART-style serial backend initialization path for x86_64 bring-up.

## Initial Target
- COM1-compatible early serial path
- fixed debug strings
- QEMU-friendly development workflow
- minimal earliest viable UART control path

## UART Initialization Goals
- disable interrupts initially
- enable divisor latch
- configure baud divisor
- configure line control
- enable FIFO policy later
- prepare transmitter path

## Engineering Rule
Keep UART logic inside serial backend and keep raw port access inside x86_64 port I/O abstraction.
