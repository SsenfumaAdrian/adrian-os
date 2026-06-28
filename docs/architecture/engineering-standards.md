# ADRIAN OS Engineering Standards

## Required for Every Module
- architecture documentation
- design rationale
- public interface documentation
- security review
- testing strategy
- performance considerations
- migration and evolution notes

## Language Policy
- Rust: kernel, drivers, security, systems services
- Dart: app platform, UI toolkit, shells, apps
- Python: tooling, automation, AI, test infrastructure only

## Quality Policy
- no undocumented public APIs
- no unjustified unsafe Rust
- no privileged service without threat model
- no hot path without performance review
