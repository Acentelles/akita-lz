# Root opening basis 5 for small dense sources

Implementation and measurements against `2f456bc99` on
`quang/root-basis-5-dense-small`. All compilation and execution use the supplied
Boat runner, with `CARGO_BUILD_JOBS=1` and catalog parallelism 4.

## Result and recommendation

Both target presets ship root opening log basis 5 with the floor disabled and
one root inner digit at every arity 13 through 29. The measurements do not favor
5 as a universal root pin. For the CKKS master, prefer **K=6** to admit unsigned
chunks `[0,31]` directly. For a subsequent byte-oriented preset, prefer **root 3
with the floor off**: V5 selects 3 for K=6 at every sampled arity. This branch
retains the requested root-5 target; the other policies remain report-only.

| K | Arity | V0 modeled bytes | V3 modeled bytes | V5 modeled bytes | V5 root |
|---:|---:|---:|---:|---:|---:|
| 5 | 19 | 68367 | 66394 | 65074 | 3 |
| 6 | 19 | 68367 | 66394 | 65634 | 3 |
| 5 | 24 | 70995 | 69703 | 68367 | 3 |
| 6 | 24 | 70995 | 69703 | 68367 | 3 |

V3 saves 2.89% over V0 at arity 19 and 1.82% at arity 24. Root 5 without removing
the floor is materially worse: V4 is 72789 and 77627 bytes at those arities for
both widths. Removing the floor from that root-5 policy saves 8.79% and 10.21%.
K=5 can save another 560 modeled bytes over K=6's V5 choice at arity 19, but
requires the centered encoding and evaluation adjustment described below.
K=5's V5 choices also vary with arity (6 at 16, 4 at 20, 3 elsewhere sampled).

The current full-field V0 already searches its inner basis separately: it uses
10 inner digits at arity 19 and 12 at arity 24, not 43. The small-source root
reduces those counts to one, but the terminal response dominates the final proof.
At these arities root 5 does not eliminate another fold relative to the best
small-source root-3 schedules, and its extra range-proof traffic remains.

The five measured runs all verified. K=6 V3 used 63756 real bytes and 0.101 s
for commit plus opening, compared with V0's 65721 bytes and 0.140 s. K=6 V1 used
64421 bytes and 0.097 s. V2/V5 were not timed, so the root-3/floor-off recommendation
is based on modeled bytes, not a measured runtime claim. These single samples
are not a statistically established speed ranking.

Base 32 retains the existing two-stage range path. Stage 1 accounts for 22.5%
(K=5) and 27.3% (K=6) of V3 opening time, or 21.0% and 25.1% including commit.
No base-32 single-stage fast path was added. The observed shares identify its
maximum relevant fraction of these runs, not a measured benefit from such a path.

## Policy and compatibility

`PlannerPolicy::root_opening_log_basis` defaults to `None`, meaning the opening
range minimum. `monotone_opening_basis` defaults to `true`. Config adapters
forward both options. Non-default options append a tagged
`AKITA-OPENING-POLICY-V2` extension to the policy digest preimage; default
policies retain exactly the V1 preimage. The artifact identity already binds
that digest. No artifact wire-format version changes.

Early regeneration of `fp128_dense_small_3` and `fp128_dense_small_4` into a
remote temporary directory passed generator check mode and bytewise `cmp`.
K=3 and K=4 also remain byte-identical after the final generation. All four
DenseSmall families receive new TSV baseline rows because none existed in the
baseline: two unchanged arity-14 rows plus 34 target rows. Existing TSV rows are
preserved verbatim.

The brief uses the old names `D64Full` and `fp128_d64_full`. At this baseline,
the corresponding live full-field preset is `fp128::Dense`, catalog
`fp128_dense`, with adaptive dimensions. Experiments use that current preset.

## Monotonicity audit

The floor is a search restriction. No protocol dependency requiring a
non-decreasing opening basis was found; no change to `akita-params` is needed.

| Component | Evidence and consequence |
|---|---|
| Search and memo | `crates/akita-planner/src/schedule_params/suffix_dp/candidates.rs:503` binds a recursive inner basis to `state.current_lb`. `search.rs:84` and `search.rs:125` pass the selected opening basis to the child. `state.rs:125` includes `current_lb` in `ScheduleMemoKey`. The key schema and bounded basis domain do not grow; reachable states and transitions can grow when the floor is disabled. |
| Prover | `crates/akita-prover/src/protocol/prove/suffix.rs:55` selects each scheduled step, checks its input length, and passes its own parameters to `prove_fold`. The terminal path uses its separately scheduled parameters. No comparison with the previous opening basis occurs. |
| Verifier | `crates/akita-verifier/src/fold/suffix.rs:83` reconstructs replay from the current level's parameters and its successor. Per-level witness lengths and commitments are checked independently of an ordering on bases. |
| Witness layout and contraction | `crates/akita-params/src/witness.rs:1020` computes Z/E/T widths from the current inner, fold, opening and outer digit counts. `crates/akita-planner/src/schedule_params/suffix_dp.rs:52` compares input and output bit volumes using their separate bases; it does not assume an ordering. |
| Norms and SIS | `crates/akita-schedules/src/audit.rs:316` recomputes the A collision bucket from the current response basis and depth. `crates/akita-params/src/sis/norm_bound.rs:206` prices the diameter of exactly that accepted digit interval. Honest child source moments are propagated in `crates/akita-planner/src/schedule_params/suffix_dp/source.rs:52`. |
| Range and terminal | `crates/akita-params/src/layout/digit_range.rs:145` constructs a plan from a single basis. `crates/akita-params/src/schedule.rs:155` derives terminal source norms from its own inner basis. Terminal response admission is recomputed independently in `crates/akita-schedules/src/audit.rs:397`. |
| Byte model | `crates/akita-schedules/src/runtime.rs:682` recomputes proof bytes from per-level layouts and the terminal response; it does not use differences between successive bases. |
| Artifact admission | `crates/akita-schedules/src/audit.rs:26` validates each digit pair, and `audit.rs:260` recomputes canonical digit depths. `crates/akita-params/src/schedule.rs:358` validates topology and successor capacities, not cross-level basis monotonicity. |

The within-level constraint that the opening alphabet covers the outer digits
(`crates/akita-params/src/layout/params.rs:325`) remains intact. It does not
constrain the predecessor's opening basis.

### Origin of the floor

`git log -S"max(state.current_lb)"` reaches #394 (`8e552d2ac`), but its parent
already has `if open_lb < current_lb { continue; }`. #355 (`9c89019b4`)
renamed the old single-basis variable while separating source and opening bases.
Following `git log -S"lb < current_lb" --all -- src crates` reaches #50
(`7e79bde4e`, 2026-04-16), which introduced this skip in
`src/planner/schedule_params.rs` within its memoized suffix search. #64 (`1b9db64df`) moved it during crate decomposition; #301 and #394
carried it through later refactors.

At introduction, the selected basis was shared by source and opening
representations: excluding a basis smaller than the incoming one avoided
representing incoming digits in a smaller single-digit alphabet. This is a
code-based inference. The commit message describes proof-size optimization
but gives no separate security rationale for the skip. Once #355 separated
source and opening bases, the source equality remained structural while the
opening restriction was retained. The new opt-in policy removes only that
retained opening search restriction.

## Small-source soundness and the inner route

Commit-time source scans constrain the supplied honest prover implementation.
They do not constitute a proof that a malicious prover's original coefficients
lie in the declared small interval. Stage 1 range-checks the **new folded witness**,
including the Z response digits, E opening digits and T commitment digits; it
does not directly range-check every original root source coefficient. See
`crates/akita-prover/src/protocol/prove/fold/sumcheck.rs:66` and
`crates/akita-verifier/src/stages/stage1.rs:203`. The surrounding relation proof
binds those range-checked responses to the commitment.

The PCS's existing SIS weak-binding argument uses the verifier-accepted response
envelope. For the Linf route,
`crates/akita-params/src/sis/norm_bound.rs:206` prices the exact diameter of the
accepted balanced response interval, multiplied by response chunks and challenge
mass. `crates/akita-schedules/src/audit.rs:316` recomputes that price and rejects
an insufficient A table bucket. For the L2 route, the schedule fixes a physical
response norm cap, the proof enforces it, and the audit reconstructs its matrix
admission from that same cap. The nonterminal verifier reads the scheduled L2 cap at
`crates/akita-verifier/src/stages/stage1.rs:247`; the cap check is in
`crates/akita-verifier/src/stages/physical_l2_norm.rs:162`. Terminal responses
are checked directly at `crates/akita-verifier/src/terminal/direct.rs:178`, with
the Linf cap admitted at `crates/akita-verifier/src/fold/terminal.rs:143`. Honest-source concentration and energy models affect
completeness and grinding cost; they do not enlarge the verifier's accepted cap.
Thus the small-source declaration is not an additional malicious-prover range
proof, and no binding argument may substitute the declared source bound for the
response bound actually enforced.

CKKS must separately establish its chunk-range semantics and bind those chunks
to the PCS statement. K=6 admits unsigned `[0,31]` directly. K=5 admits
`[-16,15]`; use `chunk - 16`, and consistently adjust claimed evaluations and
any outer arithmetic relation. For a multilinear Lagrange opening, the weights
sum to one, so this shift subtracts 16 from the evaluation. The PCS alone does
not prove that an alleged chunk came from a CKKS encoding.

### Why 835380 at K=5 exceeds 507780 at K=6

The original arity-14 rows at `2f456bc99` both use A dimension 256, root opening
log basis 3 and fold digit depth 4, but choose different packing subrings:

| K | Challenge dimension | Challenge `(count_pm1,count_pm2)` | L1 mass | A collision bound |
|---|---:|---|---:|---:|
| 5 | 64 | (31,10) | 51 | 835380 |
| 6 | 128 | (31,0) | 31 | 507780 |

A four-digit base-8 interval has diameter `8^4 - 1 = 4095`.
`rounded_up_role_a_inf_norm` prices `4 * challenge_mass * diameter` for one
response chunk. These two products are exactly 835380 and 507780; neither
requires an additional table round-up. The apparent inversion compares different
challenge geometries, not a monotone function of source width. No bug or params
change is required. The config regression
`small_dense_collision_buckets_reflect_challenge_mass_not_source_width` pins both
values against the canonical norm routine and challenge configurations.

### One root digit at every supported arity

Changing only the root opening basis let the planner choose two inner digits at
arity 14. To meet the one-digit requirement, K=5 and K=6 now start their inner
search domain at K, with the existing maximum 16. The raw root-source search
caps this at K, so the selected root inner basis is K and the digit count is 1.
Recursive `BalancedDigits` still selects the predecessor's opening basis exactly,
independently of that root domain. K=3 and K=4 retain their old policy and rows.
All small-source experiment variants use this same one-digit root domain.

## Measurements

The report harness is `crates/akita-planner/examples/root_basis_report.rs`.
The complete sweep uses `crates/akita-planner/examples/root_basis_report.py`;
the five measured runs and trace extraction use
`crates/akita-pcs/tests/akita_fp128_e2e/root_basis_measurement.py`. The latter
invokes the ignored `small_dense_measurement` test explicitly, one process per
variant, with `RAYON_NUM_THREADS=8`. All those manual tests passed.
Invoke it remotely as `cargo run --release -p akita-planner --features catalog-gen
--example root_basis_report -- K VARIANT ARITY`. K=0 selects the full-field
baseline. Report-only artifacts for the arity-19 proving runs are written under
`/tmp/akita-root-basis-report`, never to the shipped artifact directory.

V0 is full-field/root 3/floor on; V1 is small/root 3/floor on; V2 is small/root
3/floor off; V3 is small/root 5/floor off; V4 is small/root 5/floor on. V5 runs
the unchanged planner separately with each root in [3,6], floor off, and chooses
the least modeled proof bytes among the feasible selected schedules (ties choose
the smaller root). This is a byte comparison of four policy optima. The current
planner's within-root selection first minimizes direct setup capacity, then
proof/work cost; it is not an unconstrained proof-bytes-only optimization.

In the table, z/e/t are **terminal transmitted response** bytes: z uses the same
certified L2-aware Golomb estimate as the planner, while e/t are full field
encodings. Earlier Z/E/T vectors become recursively committed witnesses and are
not transmitted directly. `Other` includes their relation and sumcheck proofs,
commitment payloads, framing and modeled nonce bytes, so `Total = z + e + t +
Other`. These are modeled sizes, not measurements of serialized proofs.

Each level is `(inner log basis, opening log basis)/inner digits,opening digits,
outer digits,fold digits`. The last entry has suffix T for the direct terminal;
its second basis is the direct fold/response basis, and its e/t fields are
transmitted directly, hence the two dashes. V0 is shared between the K=5 and
K=6 comparisons.

Table planner seconds are elapsed search time under up to four concurrent
report jobs; the isolated floor timing comparison below is the wall-time
comparison to use for the floor itself.

| K | Variant | Arity | Total bytes | z | e | t | Other | Planner s | Levels |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 0 | V0 | 13 | 64380 | 22229 | 7168 | 21504 | 13479 | 3.708 | (15,3)/9,43,43,7; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 16 | 67325 | 22229 | 7168 | 21504 | 16424 | 7.305 | (13,3)/10,43,43,7; (3,3)/1,43,43,3; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 19 | 68367 | 22243 | 7168 | 21504 | 17452 | 13.366 | (13,3)/10,43,43,7; (3,3)/1,43,43,3; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 20 | 68367 | 22243 | 7168 | 21504 | 17452 | 9.681 | (12,3)/11,43,43,7; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 21 | 68483 | 22243 | 7168 | 21504 | 17568 | 14.254 | (12,3)/11,43,43,7; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 24 | 70995 | 22243 | 7168 | 21504 | 20080 | 10.311 | (11,3)/12,43,43,7; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 0 | V0 | 29 | 72371 | 22243 | 7168 | 21504 | 21456 | 20.760 | (16,3)/8,43,43,9; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.055 | (5,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 5 | V1 | 16 | 64550 | 21984 | 7168 | 21504 | 13894 | 0.933 | (5,3)/1,43,43,4; (3,4)/1,32,32,2; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 19 | 65074 | 22243 | 7168 | 21504 | 14159 | 4.341 | (5,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 20 | 67325 | 22229 | 7168 | 21504 | 16424 | 3.763 | (5,3)/1,43,43,5; (3,3)/1,43,43,3; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 21 | 67571 | 22243 | 7168 | 21504 | 16656 | 4.513 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 3.226 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V1 | 29 | 71227 | 22243 | 7168 | 21504 | 20312 | 5.797 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 5 | V2 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.124 | (5,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 5 | V2 | 16 | 64550 | 21984 | 7168 | 21504 | 13894 | 2.181 | (5,3)/1,43,43,4; (3,4)/1,32,32,2; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V2 | 19 | 65074 | 22243 | 7168 | 21504 | 14159 | 8.527 | (5,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V2 | 20 | 65831 | 22243 | 7168 | 21504 | 14916 | 7.952 | (5,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V2 | 21 | 65947 | 22243 | 7168 | 21504 | 15032 | 10.336 | (5,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V2 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 8.358 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V2 | 29 | 71227 | 22243 | 7168 | 21504 | 20312 | 14.550 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 13 | 51467 | 22528 | 5120 | 20480 | 3339 | 0.669 | (5,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 16 | 65819 | 21999 | 7168 | 21504 | 15148 | 1.345 | (5,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 19 | 66394 | 22243 | 7168 | 21504 | 15479 | 4.724 | (5,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 20 | 66394 | 22243 | 7168 | 21504 | 15479 | 7.000 | (5,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 21 | 67961 | 22243 | 7168 | 21504 | 17046 | 6.369 | (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 24 | 69703 | 22243 | 7168 | 21504 | 18788 | 12.268 | (5,5)/1,26,26,3; (5,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V3 | 29 | 72761 | 22243 | 7168 | 21504 | 21846 | 9.658 | (5,5)/1,26,26,4; (5,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V4 | 13 | 51467 | 22528 | 5120 | 20480 | 3339 | 0.160 | (5,5)/1,26,26,2; (5,6)/1,-,-,2T |
| 5 | V4 | 16 | 68729 | 24210 | 6144 | 24576 | 13799 | 0.223 | (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V4 | 19 | 72789 | 23952 | 6144 | 24576 | 18117 | 0.544 | (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V4 | 20 | 73256 | 24203 | 6144 | 24576 | 18333 | 1.028 | (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V4 | 21 | 74993 | 23967 | 6144 | 24576 | 20306 | 1.017 | (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V4 | 24 | 77627 | 24203 | 6144 | 24576 | 22704 | 1.506 | (5,5)/1,26,26,3; (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V4 | 29 | 81712 | 23952 | 6144 | 24576 | 27040 | 1.036 | (5,5)/1,26,26,4; (5,5)/1,26,26,3; (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 5 | V5 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.910 | (5,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 5 | V5 | 16 | 64431 | 24576 | 7168 | 28672 | 4015 | 6.312 | (5,6)/1,22,22,2; (6,3)/1,-,-,4T |
| 5 | V5 | 19 | 65074 | 22243 | 7168 | 21504 | 14159 | 23.737 | (5,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V5 | 20 | 65754 | 22243 | 7168 | 21504 | 14839 | 29.833 | (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V5 | 21 | 65947 | 22243 | 7168 | 21504 | 15032 | 32.224 | (5,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V5 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 42.279 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 5 | V5 | 29 | 71227 | 22243 | 7168 | 21504 | 20312 | 48.709 | (5,3)/1,43,43,5; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V1 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.041 | (6,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 6 | V1 | 16 | 64380 | 22229 | 7168 | 21504 | 13479 | 1.494 | (6,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V1 | 19 | 66993 | 22256 | 7168 | 21504 | 16065 | 3.448 | (6,3)/1,43,43,5; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V1 | 20 | 67325 | 22229 | 7168 | 21504 | 16424 | 2.809 | (6,3)/1,43,43,5; (3,3)/1,43,43,3; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V1 | 21 | 67571 | 22243 | 7168 | 21504 | 16656 | 4.794 | (6,3)/1,43,43,5; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V1 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 3.515 | (6,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V1 | 29 | 71226 | 22243 | 7168 | 21504 | 20311 | 5.630 | (6,3)/1,43,43,6; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,5)/1,-,-,3T |
| 6 | V2 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.085 | (6,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 6 | V2 | 16 | 64380 | 22229 | 7168 | 21504 | 13479 | 3.294 | (6,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V2 | 19 | 65634 | 22229 | 7168 | 21504 | 14733 | 7.829 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V2 | 20 | 65831 | 22243 | 7168 | 21504 | 14916 | 6.287 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V2 | 21 | 65947 | 22243 | 7168 | 21504 | 15032 | 10.650 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V2 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 8.417 | (6,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V2 | 29 | 71226 | 22243 | 7168 | 21504 | 20311 | 13.953 | (6,3)/1,43,43,6; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 13 | 56587 | 22528 | 6144 | 24576 | 3339 | 0.641 | (6,5)/1,26,26,3; (5,3)/1,-,-,4T |
| 6 | V3 | 16 | 65819 | 21999 | 7168 | 21504 | 15148 | 1.307 | (6,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 19 | 66394 | 22243 | 7168 | 21504 | 15479 | 4.515 | (6,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 20 | 66394 | 22243 | 7168 | 21504 | 15479 | 7.071 | (6,5)/1,26,26,3; (5,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 21 | 67961 | 22243 | 7168 | 21504 | 17046 | 6.239 | (6,5)/1,26,26,3; (5,6)/1,22,22,2; (6,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 24 | 69703 | 22243 | 7168 | 21504 | 18788 | 12.191 | (6,5)/1,26,26,3; (5,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V3 | 29 | 72761 | 22243 | 7168 | 21504 | 21846 | 8.955 | (6,5)/1,26,26,4; (5,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V4 | 13 | 56587 | 22528 | 6144 | 24576 | 3339 | 0.156 | (6,5)/1,26,26,3; (5,6)/1,-,-,2T |
| 6 | V4 | 16 | 68729 | 24210 | 6144 | 24576 | 13799 | 0.213 | (6,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V4 | 19 | 72789 | 23952 | 6144 | 24576 | 18117 | 0.511 | (6,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V4 | 20 | 73256 | 24203 | 6144 | 24576 | 18333 | 0.992 | (6,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V4 | 21 | 74993 | 23967 | 6144 | 24576 | 20306 | 0.669 | (6,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V4 | 24 | 77627 | 24203 | 6144 | 24576 | 22704 | 1.443 | (6,5)/1,26,26,3; (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V4 | 29 | 81712 | 23952 | 6144 | 24576 | 27040 | 0.649 | (6,5)/1,26,26,4; (5,5)/1,26,26,3; (5,5)/1,26,26,3; (5,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,22,22,2; (6,6)/1,-,-,3T |
| 6 | V5 | 13 | 45158 | 18432 | 6144 | 18432 | 2150 | 0.829 | (6,3)/1,43,43,4; (3,3)/1,-,-,3T |
| 6 | V5 | 16 | 64380 | 22229 | 7168 | 21504 | 13479 | 8.062 | (6,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V5 | 19 | 65634 | 22229 | 7168 | 21504 | 14733 | 21.544 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V5 | 20 | 65831 | 22243 | 7168 | 21504 | 14916 | 26.569 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V5 | 21 | 65947 | 22243 | 7168 | 21504 | 15032 | 28.395 | (6,3)/1,43,43,5; (3,5)/1,26,26,2; (5,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V5 | 24 | 68367 | 22243 | 7168 | 21504 | 17452 | 41.629 | (6,3)/1,43,43,5; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |
| 6 | V5 | 29 | 71226 | 22243 | 7168 | 21504 | 20311 | 46.924 | (6,3)/1,43,43,6; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,3)/1,43,43,4; (3,4)/1,32,32,3; (4,5)/1,26,26,2; (5,3)/1,-,-,4T |


### Existing full-field catalog, floor-off comparison

| Arity | Floor on bytes | Floor off bytes | Delta |
|---:|---:|---:|---:|
| 16 | 67325 | 65830 | -1495 (-2.22%) |
| 24 | 70995 | 69985 | -1010 (-1.42%) |

### Isolated planner wall time

Planning all 17 rows of DenseSmall<6> catalogs (arities 13..29), root 5, sequential
processes with serial row planning, no compilation included. Floor on runs first.

| Floor | Seconds |
|---|---:|
| on | 11.773 |
| off | 107.170 |


### Arity-19 measured commit/open/verify

One run per variant on the 8-vCPU / 16-GB Boat box, release build, eight Rayon
threads, Blake2b transcript. Timers exclude setup construction and the independent
evaluation oracle. Commit and opening are separate timers; prover total is their sum.
Stage 1 is the sum of inclusive `stage1_sumcheck` span durations across nonterminal
levels, collected with INFO tracing during the same run. It includes the range tree
and any fused norm proof. These are single samples, not confidence intervals.
The inputs derive from the same deterministic 5-bit chunks: K=5 subtracts 16;
V0 and K=6 use the unsigned values. Input construction/import is outside the
timers. Stage-1 comparisons use different planned layouts, so they do not
isolate the base-32 kernel at fixed geometry.

| K | Variant | Real proof bytes | Commit s | Open s | Prover total s | Verify s | Stage 1 s | Stage 1 / open | Stage 1 / total |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | V0 | 65721 | 0.013 | 0.128 | 0.140 | 0.0058 | 0.035 | 27.2% | 24.7% |
| 5 | V1 | 62439 | 0.005 | 0.092 | 0.097 | 0.0037 | 0.024 | 26.2% | 24.8% |
| 5 | V3 | 63737 | 0.008 | 0.108 | 0.116 | 0.0036 | 0.024 | 22.5% | 21.0% |
| 6 | V1 | 64421 | 0.007 | 0.091 | 0.097 | 0.0035 | 0.024 | 26.5% | 24.7% |
| 6 | V3 | 63756 | 0.008 | 0.093 | 0.101 | 0.0047 | 0.025 | 27.3% | 25.1% |


## Reproduction and validation

All Cargo, formatter, generation, test and measurement commands ran on Boat
through `.remote/run`; no local `target/` was created. The two Python report
drivers run remotely after building the planner example. Full sweep command:

```sh
.remote/run 'cargo build --release -p akita-planner --features catalog-gen --example root_basis_report && AKITA_SCHEDULE_GEN_JOBS=4 python3 crates/akita-planner/examples/root_basis_report.py'
.remote/run 'python3 crates/akita-pcs/tests/akita_fp128_e2e/root_basis_measurement.py'
```

Long runs redirect output to `/tmp/akita-*.log` on the remote host; monitoring
wrappers preserve the command exit status. Comparison artifacts, traces and
per-variant logs stay under `/tmp/akita-root-basis-report`. Only the target
catalogs and DenseSmall TSV rows are committed.

Required harness checks, with final outcomes:

| Command | Result |
|---|---|
| `test ! -d target` | PASS |
| `.remote/run 'cargo fmt --all -- --check'` | PASS |
| `.remote/run 'cargo clippy -p akita-config -p akita-planner -p akita-schedules -p akita-params -p akita-pcs --lib --tests -- -D warnings'` | PASS |
| `.remote/run 'cargo test --release -p akita-planner --lib'` | PASS: 87 passed, 3 existing manual tests ignored |
| `.remote/run 'cargo test --release -p akita-schedules --lib'` | PASS: 15 passed |
| `.remote/run 'cargo test --release -p akita-config --test small_dense_source --test trusted_schedule_artifact'` | PASS: 3 + 23 passed |
| `.remote/run 'cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features --features parallel,transcript-blake2b small_dense'` | PASS: roundtrips and rejection checks; manual measurement ignored here and separately executed five times |
| `.remote/run 'bash .remote/drift.sh'` | PASS: 27 byte-identical regenerated catalogs; 151 equal TSV rows, zero added/removed/changed; 1005.86 s |
| `.remote/existing_unchanged.sh` | PASS |
| `test -s docs/notes/root-basis-5-dense-small.md` | PASS |

Additional checks and iteration history:

- The early generator command used `cargo run --release -p akita-planner
  --features catalog-gen,catalog-check --bin gen_schedule_artifacts -- "$out"
  --check-catalog fp128_dense_small_3 fp128_dense_small_4`, followed by `cmp`
  for both files. The first attempt failed because check mode needs copies of
  the existing artifacts in its output directory. After seeding the temporary
  directory, generation, check mode and both comparisons passed.
- `.remote/run 'cargo test --release -p akita-planner --lib policy::tests'`
  initially failed to compile the obsolete preset name from the brief. Using
  the current `fp128::Dense` name passed the policy regression.
- Formatter checks caught line wrapping during editing; each was fixed remotely
  and the final full check passed. The first Clippy attempt caught a missing
  `Clone` derive on the report-only config; it was added and both subsequent
  full Clippy runs passed.
- DenseSmall generation used `AKITA_SCHEDULE_GEN_JOBS=4 cargo run --release
  -p akita-planner --features catalog-gen --bin gen_schedule_artifacts --
  artifacts/schedules fp128_dense_small_3 fp128_dense_small_4 fp128_dense_small_5
  fp128_dense_small_6 --catalog-snapshot /tmp/akita-small.tsv`. The final
  generation passed, with K=3/4 byte-identical and 36 new baseline rows.
- Both report-driver commands above passed. The measurement driver ran
  `cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features
  --features parallel,transcript-blake2b small_dense_measurement -- --ignored
  --nocapture` with `(AKITA_MEASURE_K, AKITA_MEASURE_VARIANT)` equal to
  `(0,0), (5,1), (5,3), (6,1), (6,3)`. All five passed.
- The initial remote cheap preflight passed 179 Python tests, Rust line-cap
  self-tests, config/planner dependency hygiene, external artifact hygiene,
  and shared field/polynomial identity checks. Its aggregate exit was 127:
  `taplo` and `typos` are absent, and the tracked-file line-cap gate requires
  git metadata, which the remote checkout intentionally lacks. No local
  compilation or test fallback was used.

- `.remote/run './scripts/check-doc-guardrails.sh && cargo clippy -p akita-planner --example root_basis_report --features catalog-gen -- -D warnings'` passed. All four documentation reference/path gates passed; the script skipped the optional book build because `mdbook` is absent. The report example passed Clippy with warnings denied.
