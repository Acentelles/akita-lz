# Spec: One planner policy for automatic setup offloading

| Field | Value |
| --- | --- |
| Author(s) | Akita maintainers |
| Created | 2026-10-15 |
| Status | active |
| PR | Pending |
| Supersedes | Earlier uncommitted root-threshold proposal |
| Superseded-by | |
| Book-chapter | book/src/how/configuration.md |

## Summary

Applications currently choose a direct-only configuration or a recursive
setup-offloading companion. The offloading-enabled planner already compares
complete schedules containing zero, one, or two setup offloads. Make that
planner the sole configuration. Remove the separate direct-only policy,
adapter, catalog families, profiling modes, and CI settings. Keep direct setup
evaluation wherever the selected schedule has no outgoing setup offload.
There is no new size threshold and no requirement to offload at the root.

## Intent

### Goal

Use the existing setup-envelope-first objective for every configuration and
let offline planning choose the setup-offload edges.

### Invariants

- Preserve the existing offloading-enabled objective and feasibility checks.
- Keep the production search bound: only producer levels zero and one may
  offload setup. Two is a maximum, not an exact number.
- Zero-offload schedules remain candidates, including for small inputs.
- Preserve direct setup-contribution evaluation and terminal authentication; removing a
  public mode must not remove these necessary protocol operations.
- Runtime continues to load independently audited trusted catalog rows. Proofs
  cannot choose a schedule, and planner search stays offline.
- Config, catalog identity, descriptor binding, prover, and verifier agree.
  Retired direct-only artifacts are rejected; objective tags are never reused.
- Preserve supported source contracts, witness chunk policies, and grouped
  workload shapes while consolidating duplicate catalog families. Grouped keys
  must use the independent commitment profiles selected by the unified scalar
  catalog. Regenerating those profiles can change commitment and proof bytes.

### Non-goals

No root-size dispatch threshold, deeper offloading, new cost model, or verifier
latency claim. Do not force every large input to use exactly two offloads.

## Evidence and interpretation

The baseline survey at fddf785cf1d6011b508c271d78e69958d1fde4af contains
584 cells across 60 field/source/chunk combinations: 561 successful results,
23 planning timeouts, and no admission errors. Sixteen results reuse committed
catalog rows. Generated results passed independent catalog admission. The raw
research records are excluded from the PR; representative observations follow.

All 60 combinations have both zero and positive offload observations; 46 have
two-offload observations. Small inputs commonly choose zero, but a universal
size threshold is not the current policy. The lexicographic objective compares
padded setup capacity, first directly evaluated setup capacity, exact modeled
cost, proof bytes, first-direct output witness, and descriptor. Discrete ring,
split, padding, source, and chunk choices can change the winning geometry.
The survey measures selected schedules, not each losing candidate's cause.

Here nv means one polynomial has 2^nv source entries. These ranges contain
every integer tested; separated points do not fill the intervening gaps.

| Unchunked source | Zero offloads | One offload | Two offloads |
| --- | --- | --- | --- |
| fp32 dense | 16–22 | 23–32, 40 | 44, 48 |
| fp32 one-hot | 16–24 | 25–32, 39–40 | 44, 48 |
| fp64 dense | 16–21 | 22–32, 36 | 38 |
| fp64 one-hot | 16–24 | 25–32, 39 | 40, 48 |
| fp128 dense | 16–24 | 25–32 | 36, 40 |
| fp128 one-hot | 16–24, 26, 28 | 25, 27, 29–32, 36 | 40, 42, 48 |

The first five scalar series are monotone on their successful sampled points;
that is not a global guarantee. fp64 dense timed out at 40 and 48, and fp128
dense at 48. fp128 one-hot is non-monotone even at adjacent inputs:
24:0, 25:1, 26:0, 27:1, 28:0, 29:1, under one policy digest.
For fp128 one-hot with eight witness chunks over the first two levels, every
integer nv=16…48 was checked: zero at 16–28, one at 29–34, two at 35–48.
Other chunk combinations have different observed boundaries; these results do
not establish a single threshold across fields, sources, and layouts.

Planning durations are offline search diagnostics under concurrent load.
Proof sizes are modeled payload estimates. Neither is a measured verifier
speedup. Timeouts establish no offload count or infeasibility claim.

## Design

Remove the user-selectable direct/recursive distinction from configuration and
planner policy. Base presets directly own the sole offloading-enabled catalog
family. Remove the companion adapter and retired objective implementation.
Consolidate scalar and grouped catalog coverage, regenerate canonical artifacts,
and update consumers, profiling scripts, report parsers, and CI together.
Retain the zero-offload branch within suffix search and the shared direct setup
capacity metrics used by the surviving objective.

Catalog migration preserves all 27 workload shapes from the former offloading
companions. Twenty-five schedules and their frozen profiles are identical after
family consolidation. The two W2R3 grouped rows use a newly selected nv16
precommit profile: A dimension 256 becomes 512, B dimension 128 becomes 64,
and outer slicing changes from four to two. Their complete keys and schedules
therefore change. This is an intentional catalog/configuration break; callers
must regenerate setup and commitments from the new catalogs. The offloading
objective and its policy digest are unchanged for all eight former companion
families.

## Evaluation

- [x] Only one public setup-offloading policy and configuration family remains.
- [x] No direct-only mode or settings remain in active code, profiles, or CI.
- [x] Zero, one, and two offloads are covered; production search excludes deeper
      producer levels, and every generated row passes independent admission.
- [x] Surviving planner choices match prior offloading-enabled choices for
      identical inputs, policy, and commitment profiles.
- [x] Catalog regeneration, admission/security checks, round trips, and
      malformed-artifact rejection tests pass for affected families.
- [x] CI feature matrices and profiling scripts use the consolidated families.
- [x] Book and PR explain zero-offload eligibility, non-monotonicity, all survey
      limitations, representative scalar data, and chunked results.

Run cheap repository gates before expensive builds, then focused config,
schedule/planner and PCS tests, the CI feature graphs, and affected profiling
script tests. Compare regenerated rows against old recursive families using
normalized schedule content, excluding intentional family/policy identities.

## Alternatives considered

A fixed root threshold would duplicate a decision the existing planner already
makes and would suppress valid non-monotone choices. Keeping two public modes
would preserve the unnecessary application choice. Expanding the search past
two offloads changes scope and is explicitly excluded.

## Documentation and execution

Update configuration and setup-offloading Book chapters, profiling usage,
crate READMEs, live planner specs, and deleted-symbol guards. Commit this design
record before implementation. Summarize the survey findings without committing
the raw research records or survey artifacts.
Prepare a PR description that states the API break, catalog migration,
unchanged two-offload maximum, exact objective, data, validation, and limits.

## Protocol fixture follow-up

The portable tensor-transfer regression pins an explicitly admitted nv16,
two-polynomial, zero-offload schedule independently of the optimizer. Keep the
separate offloaded-route regression as well. Follow-up: pin similarly small
admitted schedules for the quotient/reduced replay and compressed/raw tampering
regressions, so optimizer changes do not require moving their workload sizes.
Planner-choice tests should continue to exercise the optimizer itself.
