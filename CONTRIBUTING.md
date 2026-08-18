# Contributing to ADRIAN OS

## Before anything else

Read [PROGRESS.md](PROGRESS.md) for the current, honest state of every
subsystem — what's real, what's tested, what's deliberately deferred
and why. It's kept up to date; trust it over assumptions from a
subsystem's README alone, since several READMEs describe the eventual
scope of an area rather than its current state.

## The standard this project holds itself to

These aren't aspirational. They're the actual bar every change in this
codebase's history has been held to so far:

- **Build clean, always.** `cargo build` with zero warnings, for every
  feature combination a crate supports (`adrian-kernel` specifically
  needs checking in both its default `no_std` build and
  `--features std`).
- **Test, and read the actual output.** `cargo build` clean is
  necessary but not sufficient — it doesn't even compile `#[cfg(test)]`
  code. Run `cargo test` and read the real result line, not a
  truncated view of it (piping through `tail` has hidden real failures
  in this project's own history — don't repeat that).
- **Hand-verify before trusting an algorithm.** Wraparound logic, tree
  algorithms, boundary conditions — trace through a concrete example
  by hand before writing the test that's supposed to confirm it. A
  test you wrote to match code you didn't independently check proves
  the two agree with each other, not that either is correct.
- **Cryptographic code needs a different bar entirely.** Round-trip
  tests only prove self-consistency — a broken implementation can
  still round-trip with itself. Every primitive in `vault` is checked
  against an official RFC test vector, fetched from the source
  document itself (not typed from memory), cross-referenced against
  multiple independent sources before trusting the exact bytes. New
  cryptographic primitives should meet the same bar. And don't
  hand-roll a cipher, hash, or signature scheme — use an audited
  crate. This is the one deliberate exception to building everything
  native; it's not up for relitigating per-PR.
- **Concurrency needs its own scrutiny.** This project has shipped a
  real deadlock once (`match LOCK.lock().method() { ... }` — a lock
  guard kept alive for the whole match block by Rust's temporary
  lifetime extension, then re-locked from inside an arm). If you're
  touching anything that holds more than one lock, or calls into code
  that might, check the lock ordering is consistent everywhere, and
  actually run the test suite with a timeout rather than trust that it
  finished.
- **Verify before extending, not after.** Check what a file, type, or
  config constant already contains before adding something that might
  duplicate or conflict with it. This project has hit the same class
  of mistake twice — a redundant identifier type that duplicated an
  existing one, and a capacity constant that didn't check
  `config.rs`'s pre-existing values first — both caught and fixed, but
  both avoidable.
- **Global test state needs relative assertions.** Anything touching a
  shared global (an atomic counter, a `SpinLock`-protected static)
  runs concurrently with every other test in the binary by default.
  Assert relative properties (strictly increasing, distinct, "starts
  empty" only if genuinely nothing else in the test suite touches it)
  rather than absolute ones that assume you're the only test running.

## Practical setup

See [README.md](README.md#building--testing) for build commands.
`vault`'s dependencies are pinned to specific versions for toolchain
compatibility reasons documented in PROGRESS.md — don't bump them to
"latest" without checking whether that reintroduces the edition2024
wall this project has already hit twice.

## Commits

- Explain the *why*, not just the *what* — a commit message that only
  restates the diff wastes the one place in the repository where
  reasoning actually gets recorded.
- If verification caught something (a wrong boundary, a race, a stale
  assumption), say so. That's more useful to the next person than a
  commit that reads as if everything worked on the first attempt.
- Commits in this repository are signed. See GitHub's docs on
  [signing commits with SSH](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)
  if you haven't set this up before.

## Scope discipline

Every subsystem in this codebase draws an explicit line between real,
verifiable logic and whatever needs infrastructure that doesn't exist
yet (real hardware, real entropy, real persistent storage, a settled
design decision). Keep drawing that line rather than papering over a
gap with something that looks more finished than it is — PROGRESS.md
exists specifically so "what's actually done" stays honest and
checkable.
