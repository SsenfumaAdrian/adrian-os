# ADRIAN OS Kernel Bring-Up Plan v1

## Bring-Up Goal
Reach deterministic early kernel initialization on x86_64 with visible debug output and structured init ordering.

## Milestones
- BUP-1 kernel image reaches early entry
- BUP-2 visible debug output
- BUP-3 early init sequence runs in order
- BUP-4 memory/bootstrap structures recognized
- BUP-5 interrupt/timer stubs integrated
- BUP-6 idle/system execution context planning
- BUP-7 first userspace launch preparation

## Immediate Priorities
1. Boot context structure
2. x86_64 entry path
3. early init sequencing
4. memory region modeling
5. panic/halt path
6. QEMU run strategy
