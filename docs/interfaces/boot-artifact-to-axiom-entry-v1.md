# ADRIAN OS Boot Artifact to Axiom Entry Interface v1

## Purpose
Define the conceptual handoff between a future boot artifact and the Axiom kernel entry boundary.

## Expected Flow
1. boot artifact gains control
2. boot artifact prepares or translates handoff state
3. boot artifact provides BootContext-compatible data
4. boot artifact calls into entry::kernel_entry(&BootContext)

## Notes
Temporary experiment paths must be documented clearly if they differ from final Halo-integrated design.
