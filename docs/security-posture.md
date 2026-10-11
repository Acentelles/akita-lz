# Akita Security Posture

This document records the security boundaries that reviewers should keep in mind when changing Akita.
It is not a substitute for a third-party audit.

## Trust Boundaries

Akita has four main trust boundaries:

- Proof bytes, commitments, claims, and opening points may be attacker controlled.
- Verifier preprocessing is pinned by the deployment and trusted for soundness; see [Verifier Preprocessing](#verifier-preprocessing).
- Prover witnesses and intermediate polynomials may contain private application data.
- Build inputs, Git dependencies, and GitHub Actions influence the binary that users run.

Validated deserialization is the default for bytes that cross a trust boundary.
Unchecked deserialization is reserved for internal buffers whose producer and shape have already been validated in the same trust domain.

## Verifier Preprocessing

The verifier reads four preprocessed inputs besides the proof and the public statement:

- the expanded public matrix, a function of the setup seed;
- the setup-prefix commitment registry, a function of the seed and the schedule rows that offload setup;
- the optional terminal NTT cache, a function of the seed and one schedule row;
- the schedule catalog, which is planner output checked against the compiled SIS tables.

These inputs play the role of a verifying key.
They must be derived honestly once and then pinned by the deployment, for example by shipping them with a release, compiling them into the verifier program, or checking a digest that the verifier program commits to.
After that, a verifier either trusts the pinned data or recomputes it.
Recomputation is always allowed and never required.
Avoiding it is the reason preprocessing exists, and recomputing the setup-prefix commitments is the most expensive case.

A verifier that receives preprocessing from an adversary is outside Akita's soundness claim, in the same way as a verifier that is given the wrong verifying key.
Akita does not pin these inputs itself.
It binds the setup seed and the selected schedule row digest into the transcript, but it has no release manifest or signature for the catalog, the registry, or the cache.
Pinning is the deployment's responsibility.

Two obligations remain for trusted preprocessing:

- Parsing stays total. Malformed preprocessing bytes are rejected with an error under the no-panic boundary below, even though their content is trusted for soundness once accepted.
- Cheap admission checks are kept where an honest but faulty generator could produce data that verifies and is weaker than intended. The SIS re-audit of catalog rows is the example. A check is not needed when the fault it guards against makes every honest proof fail, because tests cover that.

Current validation is therefore uneven by design.
Validated decoding re-derives the public matrix from the seed.
Catalog admission re-audits each row.
The setup-prefix registry is checked for structure and for binding to the setup seed, and a supplied terminal NTT cache for identity, shape, and residue ranges.
Their contents are otherwise trusted.

Review findings should be classified against this model.
A hostile proof that is accepted, panics, or forces unbounded work against authentic preprocessing is a defect.
A scenario that requires hostile preprocessing is out of scope unless it also violates the no-panic boundary.

## Verifier No-Panic Boundary

Verifier-facing execution must be panic-free for malformed public inputs.
Public proofs, setup artifacts, schedules, fold parameters, opening points, commitments, claim incidence summaries, direct witnesses, and transcript data must be rejected with `AkitaError` or `SerializationError` rather than `panic!`, assertions, unchecked indexing, arithmetic overflow, or unbounded allocation.

Verifier-reachable helpers may remain infallible only when a prior boundary has established the required invariant.
That validation should live at deserialization, setup validation, schedule selection, verifier API entry, or prepared-state construction.
Hot verifier arithmetic paths should not grow fallback evaluators or repeated defensive checks when a single boundary check can preserve the existing execution model.

## Soundness-Critical Surfaces

Reviewers should treat these changes as security-sensitive:

- verifier acceptance logic,
- verifier input validation or any verifier-reachable panic-shaped operation,
- Fiat-Shamir domain labels, transcript order, or challenge derivation,
- canonical field, ring, proof, setup, and claim serialization,
- crate dependency edges into verifier-facing crates,
- configuration schedules that determine proof shape,
- unsafe field, algebra, NTT, or matrix kernels,
- dependency, toolchain, or CI changes.

## Unsafe Code Policy

Unsafe code is allowed only where it buys concrete performance or layout control that safe Rust cannot express cleanly.
Every unsafe block should have a local safety argument that names the invariant being relied on.
Verifier-facing crates should avoid unsafe code unless a spec explicitly justifies it.

## Resource Limits

Verifier-facing decoding must not allocate solely from attacker-provided lengths without an explicit bound.
When a proof shape already determines a length, prefer shape-derived allocation over self-described vector lengths.
Validated proof-shape decoding must reject shape dimensions that exceed the generic verifier-facing allocation cap before allocating shape-controlled buffers.

## Current Assurance

Akita currently relies on strict Rust CI, crate-boundary checks, specs for large protocol changes, and targeted tests.
The hardening roadmap adds supply-chain checks, fuzzing, property tests, bounded untrusted decoding, and clearer unsafe and panic discipline.
