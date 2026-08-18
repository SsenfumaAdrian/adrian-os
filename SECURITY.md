# Security Policy

## Where this project actually is

ADRIAN OS is pre-boot, early-stage software — see
[PROGRESS.md](PROGRESS.md) for the honest current state. It is not
deployed anywhere, doesn't run on real hardware yet, and nothing in
this repository should be used to protect real data or real systems
today. That said, `vault`'s cryptographic primitives are real,
RFC-verified code built on audited crates, and the kernel's capability
and IPC model is real, tested logic — reports about actual flaws in
that code are genuinely useful now, before anything depends on it.

## Reporting a vulnerability

Please don't open a public issue for a security concern. Instead:

- Use [GitHub's private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)
  for this repository (Security tab → Report a vulnerability), or
- Email **adrianssenfuma@gmail.com** with a clear description, the
  affected file(s) or commit, and, if possible, a minimal reproduction
  or proof of concept.

Include enough detail to actually verify and reproduce the issue —
what you expected, what actually happened, and why it matters
(what could go wrong as a result).

## What's actually in scope right now

- **`vault`** — anything where the wrapper code around a primitive
  (key handling, nonce handling, error propagation) could weaken the
  guarantees the underlying audited crate provides. The primitives
  themselves (ChaCha20-Poly1305, HKDF-SHA256, Ed25519, all via
  RustCrypto) are out of scope here — report those upstream — but how
  this project *uses* them is very much in scope.
- **`rian/kernel`**'s capability and IPC model — logic errors in
  `CapabilityRights::can_derive`, `is_authorized`, or the handle
  registry that could let something narrow-then-widen a capability, or
  access an object it shouldn't.
- Anything that could compromise the integrity of the build or commit
  history itself (supply-chain concerns in pinned dependencies,
  tampering-relevant gaps).

## Response expectations

This is presently a small, early-stage project without a formal
security team or SLA. Reports will be read and acknowledged as soon as
reasonably possible; fixes are prioritized by actual severity and
exploitability given the project's current stage, not by a fixed
timeline. If something is severe enough to warrant urgency, say so
explicitly in the report.

## Disclosure

Coordinated disclosure is appreciated — please allow a reasonable
window to investigate and address a report before any public
discussion. There's no bug bounty program at this stage.
