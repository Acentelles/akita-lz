# Floor-off default: blocked by catalog-generation memory

Status: **incomplete**. Baseline is `4bdbd42fa806a3286884d4450d2b95b58205505d`
on `quang/root-basis-5-dense-small`. All builds and generator execution were
remote through `.remote/run`; no local `target/` was created.

The policy source changes are committed as `a125c36c7`, but **the checked-in catalogs and
TSV are still the baseline versions**. Neither full generator run reached
publication. Those catalogs do not bind to the new policy digest. This branch
is not ready for use or merge in this state.

## Blocking result and regeneration time

Both attempts used the supplied 8-vCPU, 16-GB Boat box,
`CARGO_BUILD_JOBS=1`, `AKITA_SCHEDULE_GEN_JOBS=4`, all catalog families, and
`--catalog-snapshot artifacts/schedule-catalog.tsv`.

| Policy / runtime | Full-run outcome | Wall time | Peak RSS |
| --- | --- | ---: | ---: |
| Floor on at baseline | Completed, timing supplied by owner | about 890–1000 s | not supplied |
| Floor off, normal allocator | SIGKILL, wrapper exit 137; no publication | 1702.57 s | 15,636,136 KiB (14.91 GiB) |
| Floor off, allocator tuning | SIGKILL, wrapper exit 137; no publication | 2540.86 s | 15,668,604 KiB (14.94 GiB) |

The first wall time includes 17.34 s of compilation. Neither floor-off time
is a successful full-regeneration time. The allocator retry used
`MALLOC_ARENA_MAX=2`, `MALLOC_TRIM_THRESHOLD_=131072`, and
`MALLOC_MMAP_THRESHOLD_=131072`; these settings applied only to that command.
The signal, peak RSS, idle-box memory inspection, and repeated failure near
physical-memory capacity are consistent with OOM. Kernel logs were not readable
(`dmesg: Operation not permitted`), so the kernel's kill reason was not directly
confirmed. GNU time printed `Exit status: 0` after its signal diagnostic; the
actual `.remote/run` exit code was **137** in both attempts.

The normal run was killed after starting fp64 one-hot arity 30 while recursive
fp64 dense searches remained active. The retry got as far as starting fp64
one-hot arity 35; its last completed recursive dense row was arity 25. That row
alone retained 235,006 memo states at peak and generated 28,459,091 candidates.
The expanded search domain has a material planner time and memory cost on this
box. No planner optimization or concurrency change was made.

Raw evidence:

- [Normal run](../../artifacts/floor-off-default/regeneration.log).
- [Allocator retry](../../artifacts/floor-off-default/regeneration-trimmed.log).
- [CPU and baseline build fingerprint](../../artifacts/floor-off-default/environment.txt).

Completing the required four-worker run requires a planner memory-use change
or a larger-memory runner. Reducing generator concurrency is another possible
workaround, but would change the requested measurement and the supplied drift
check's default. The brief permits only the policy default, digest, artifacts,
tests, and this note, so those remedies were not implemented.

## Preliminary profile-case modeled results

The table below compares the baseline TSV with completed planner diagnostics
from the failed runs. **These are unpublished planner results, not regenerated
catalogs or benchmark measurements.** Missing rows were not completed. The
extra W8R2 singleton is included because it appears in the actual workflow
matrix in addition to the twelve cases listed in the brief.

| Profile case | Baseline modeled bytes | Floor-off planner result (unpublished) | Real proof bytes / prove / verify |
| --- | ---: | ---: | --- |
| `dense_fp32:30:1` | 66,974 | not completed | not run / not run / not run |
| `onehot_fp32:34:1` | 67,950 | not completed | not run / not run / not run |
| `dense_fp64:29:1` | 69,054 | 69,054 (+0.00%) | not run / not run / not run |
| `onehot_fp64:35:1` | 69,610 | not completed | not run / not run / not run |
| `dense_fp128:28:1` | 71,798 | 71,798 (+0.00%) | not run / not run / not run |
| `onehot_fp128:36:1:direct` | 72,487 | 72,487 (+0.00%) | not run / not run / not run |
| `onehot_fp128:36:1:recursive` | 75,965 | 75,965 (+0.00%) | not run / not run / not run |
| `onehot_fp128_multi_group:34:4:direct` | 72,373 | 72,349 (-0.03%) | not run / not run / not run |
| `onehot_fp128_multi_group_recursive:34:4:recursive` | 88,933 | 77,959 (-12.34%) | not run / not run / not run |
| `onehot_fp128_multi_group_recursive_multi_chunk_w8r2:32:4:recursive` | 78,139 | 76,967 (-1.50%) | not run / not run / not run |
| `onehot_fp128_multi_chunk_w2r2:32:1` | 72,255 | 72,231 (-0.03%) | not run / not run / not run |
| `onehot_fp128_multi_chunk_w4r2:32:1` | 73,235 | 72,802 (-0.59%) | not run / not run / not run |
| `onehot_fp128_multi_chunk_w8r2:32:1` | 74,659 | 74,659 (+0.00%) | not run / not run / not run |

No profile process was run: using the new binary with the old catalogs would
fail policy binding and would not measure the requested floor-off schedules.
There is consequently no real-byte or timing delta, CI-format comparison,
full catalog diff, or performance-regression conclusion. The completed large base
profiles show no modeled-byte improvement. The recursive multi-group profile
shows a preliminary 12.34% modeled reduction, and several distributed/grouped
profiles have smaller reductions. None of these results establishes a real-byte
or timing change.

## Policy and digest changes

`CommitmentConfig::monotone_opening_basis()` now defaults to `false`, as do the
manually constructed default policies in schedule tests. DenseSmall widths 3
and 4 use floor-off; widths 5 and 6 remain floor-off with root opening log basis
5. The opt-in method and planner flag remain available.

The policy preimage domain changes from `AKITA-PLANNER-POLICY-V1` to
`AKITA-PLANNER-POLICY-V2`, preventing the new default from aliasing old floor-on
catalog identities. With root override `None` and floor off, no opening-policy
extension is encoded. A root override or floor-on opt-in adds the tagged
`AKITA-OPENING-POLICY-V2` extension, followed by the optional root and the
monotonicity boolean. Artifact envelope version and protocol epoch are unchanged.

The digest regression test checks the new default, both option identities,
and restoration of the default identity when both options are reset.
DenseSmall tests now assert floor-off and the intended root overrides.
No protocol, prover, verifier, or params implementation was changed.

## Validation and remaining work

- Local `test ! -d target`, `git diff --check`, and
  `scripts/check-external-schedule-artifacts.sh` passed.
- Remote formatting passed before generation and again after the failed runs.
  The focused policy-identity test also passed (one test; 89 filtered out), as recorded in
  [the policy log](../../artifacts/floor-off-default/checks/policy.log).
- The extra preflight stopped at missing `taplo`; it was not installed.
- `git diff --quiet 4bdbd42fa -- artifacts/schedules artifacts/schedule-catalog.tsv`
  confirms that catalog publication did not occur. The requested nonempty
  catalog-diff gate therefore remains unsatisfied.
- The full required Clippy/test/security/drift matrix has **not** been completed.
  Trusted-artifact tests and runtime benchmarks need regenerated catalogs first.
- The two setup-prefix digest fixtures in
  `crates/akita-config/tests/trusted_schedule_artifact/setup.rs` still need to be
  refreshed from the regenerated catalogs, without weakening their assertions.

The baseline profile binary is anchored by its compile-time manifest path to
`/home/user/akita-baseline/artifacts/schedules`; it does not depend on the
invocation's current directory for catalog lookup. Its fingerprint records the
requested `parallel,profile-ci,transcript-blake2b` feature set and empty Rust
flags. CI additionally uses `-D warnings -C target-cpu=x86-64-v3`; that build
flag difference must be accounted for in any continuation's timing comparison.
The baseline checkout was only inspected and was not modified.

Prepared continuation tools under `artifacts/floor-off-default/`:
`catalog_diff.py` joins every catalog row and per-group basis sequence to the
baseline; `run-checks.sh` records each required remote check separately;
`run-profile.sh` invokes the unchanged CI runner and formatter with all thirteen
cases, three measured runs, one warmup, and eight prove/verify threads. They
must be run only after resolving the generator memory blocker; neither the
check runner nor the profile runner has been executed.
