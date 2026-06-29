# ADRIAN OS Boot Image to Axiom Invocation Interface v1

## Expected Conceptual Flow
1. wrapper entry gains control
2. wrapper bridge prepares BootContext-compatible state
3. wrapper invocation layer calls entry::kernel_entry(&BootContext)
4. Axiom validates and continues internal initialization

## Rule
The wrapper should adapt and invoke. The kernel should validate and initialize.
