# ADRIAN OS FBE-1 Synthetic BootContext Strategy v1

## Purpose
Define how synthetic BootContext use is allowed in the first runnable boot experiment.

## Rule
Synthetic BootContext is acceptable only as a temporary experiment aid and must not redefine long-term BootContext semantics.

## Allowed
- minimal scaffolded BootContext population
- placeholder architecture values
- placeholder memory-map summary values
- placeholder framebuffer summary values if not yet used for real display logic

## Forbidden
- redefining field meaning casually
- hiding experiment assumptions from documentation
- treating synthetic state as production-trust state
