# Spec: Whiteout on Akita `dev` (RP4: M6e, W2, W1, W3, W5)

| Field     | Value |
| --------- | ----- |
| Author(s) | @markosg04 |
| Created   | 2026-10-21 |
| Status    | proposed |
| PR        | one per slice of §8, into `dev` |
| Parent    | jolt `specs/shared-primitive-layer.md` (RP4), `specs/whiteout-interfaces.md` (§2, §5, Open questions) |
| Siblings  | jolt `specs/whiteout-sigma.md` (W4, register A1–A23), `specs/whiteout-masking.md` (M6b/M6c) |

This note is a design only. Paper labels refer to
`akita-paper/sections/whiteout/` as read on 2026-10-21. Akita references are to
this repository at `87b4ca046` (`whiteout/dev`); jolt references are to jolt
`whiteout/dev` at `591fc9d65`, written `jolt:`. "Fold j" is the paper's
numbering; Akita's code numbers the same fold **level j − 1** (the root is
level 0). The headline instance is Jolt at T = 2^24 with
`F = Prime128OffsetA7F7` (q = 2^128 − 2^32 + 22537).

## Decision

RP4 makes Akita's folds 1..k run masked and randomized, stops the opening at
fold **k = 2** (Akita level 1), and proves the Σ-protocol's response with an
ordinary Akita proof. Concretely:

- **W2.** In ZK mode a fold has no fold-response nonce. Its challenges come
  from a fork at counter 0, its digit ranges are sized for one shot at
  2^−128, and an out-of-range response returns
  `AkitaError::FoldOverflow { level }`. The caller then reruns the whole
  proof with fresh pads and randomness. Clear schedules keep the 12-bit
  search, and their bytes do not change.
- **W1.** Every outer image (B per slice, D per level) of a commitment made
  before or at fold k gets fresh randomness columns. The randomness is drawn
  with `jolt_crypto::lattice::sample_box_digits` in the witness's own digit
  alphabet and sized by `required_randomness_columns`. Its digits join the
  witness that already holds that commitment's opening digits. Clear
  schedules have zero randomness columns, so their bytes do not change.
- **W3.** Akita gets a masked fold verifier for levels 0..k−1 that emits
  `ResidualRows` through the shared `ClaimDomain` lowering. It answers the
  four open questions of interfaces §2 and the Open questions section:
  stage-3 claims are public, `response_l2_sq` and `NextWitnessInnerState`
  fall after the cut, scalar openings are `bind_derived`, and Jolt has no
  extension-field products. The cut hands W6 a typed `ResidualOpening`.
- **W5.** Akita implements `jolt_zk::whiteout::ResponseProver`. It commits
  to e, runs the ring-quadratic product reduction and Akita's range
  reduction, offloads the response relation's setup work into the opening's
  root Stage 3, and opens E(r) with the ordinary Akita opening. W5 also
  provides compression for images above 512 coefficients. The last W5 PR
  publishes the ZK config type and its schedule family. Until then, no public
  API selects ZK.
- **M6e.** M6e deletes `akita-zk-prover`, `akita-zk-verifier`, and the M1c/M5c
  re-export shims.

Section 7 lists five jolt changes (J1–J5) that must be in the jolt revision
RP4 pins. That is a change to the review-point plan (U1).

## 1. Mode, selection, and what clear mode keeps

`docs/branches.md` decides the selection mechanism. An extension is selected
at runtime, through a configuration type or a schedule. A Cargo feature only
compiles it.

- **Feature.** `zk` on `akita-params`, `akita-types`, `akita-prover`,
  `akita-verifier`, `akita-pcs`, and `akita-planner`, off by default. It
  compiles the new modules and the `jolt-zk` dependency (features `whiteout`,
  plus `whiteout-prover` on prover crates). Enabling it changes no clear
  byte (rules 1–3).
- **Selection.** `ZkCommitmentConfig<Cfg>` in `akita-config`, a type adapter
  shaped like `RecursiveCommitmentConfig<Cfg>`
  (`recursive_commitment.rs:11`). It carries a separate schedule family
  `<base>_zk` and sets `ProtocolFeatureSet { zk: true }`
  (`akita-types/src/instance_descriptor/mod.rs:135-150`). The instance
  descriptor digest therefore binds the mode into the transcript, and a
  clear verifier rejects a ZK proof at the descriptor.
- **Artifacts.** New files under `artifacts/schedules/` for the ZK family
  only (rule 6 allows additions). Before the last PR, ZK schedules exist only
  as planner test fixtures.
- **Not selectable before the last PR.** `ZkCommitmentConfig` and the ZK
  prove/verify entry points stay `pub(crate)` until W5's last PR. Tests reach
  them through each crate's `test-support`.
- **Not the dev protocol.** ZK is not part of `dev-protocol`. That feature
  replaces the grammar of every proof, while ZK adds a configuration beside
  the clear one.

**Clear-mode invariant (C0).** With any clear `Cfg`, `dev` with `zk` on
produces the same setup, commitment, and proof bytes as `main` at the merge
base. Every RP4 PR records that comparison (branches.md, "Enforcement gap").

## 2. W2: one-shot fold responses

### 2.1 The nonce today

Each fold squeezes a 32-byte seed. It then sends a 12-bit LEB128 nonce,
found by `first_jointly_accepted_nonce` (`akita-prover/src/protocol/fold_grind.rs:199-212`),
and draws its sparse challenges from `Fork::new(seed, nonce)`
(`akita-types/src/transcript_grinding/replay.rs:309-407`, verifier
`:507-523`). The acceptance predicate `admit_fold_response`
(`akita-cpu-backend/src/opaque/fold.rs:409-452`) reads the witness, so the
accepted nonce reveals how many witness-dependent rejections came first.
The digit ranges are sized for a prover that retries. Their per-attempt
acceptance targets are 1/8 (`akita-params/src/sis/fold_linf_cap.rs:14-15`),
7/8 for the one-hot Chernoff bound, and 1/40 for the modeled caps
(`akita-planner/src/response_model.rs:32-41`). Paper §6.2 confirms that the
one-shot bound is vacuous at these ranges.

### 2.2 Design

- **Plan entry.** `GrindingRun::fold_response(level)`
  (`akita-params/src/transcript_grinding.rs:454-462`) gains a policy:
  `FoldResponseSearch::{Counter { nonce_bits: 12 }, OneShot}`. Clear plans
  use `Counter`, which is today's entry, so their bytes do not change. ZK
  plans use `OneShot` for every level up to and including the cut level.
  The policy byte joins the grinding-policy bytes (`:47-48`), so the plan
  digest binds it.
- **Transcript.** Under `OneShot` the order is: site, seed squeeze,
  challenges from `Fork::new(seed, 0)`. No nonce message is sent, so a
  ZK fold's transcript has no witness-dependent public value before its
  challenges. `commit_fold_response` and `read_fold_response` branch on the
  plan entry and never on the witness. Sending a constant zero nonce would
  also work, but this design rejects it: it costs a byte per fold and leaves
  a receive the verifier must still range-check.
- **Prover.** `replay_multi_group_fold_decompose_witnesses` runs exactly one
  probe. If `probe_opening_fold` rejects at counter 0, the prover returns
  `AkitaError::FoldOverflow { level }`, a new variant that also covers the
  chunked case. It never tries counter 1. Paper H1 bounds this abort in the
  zero-knowledge error at 2δ_fold.
- **Errors.** `FoldOverflow` is retryable, and it is the only retryable
  Akita error. The opening prover does not retry, because it does not own
  the pads. W6 maps the error to `ZkError::FoldOverflow` (J5). The jolt
  prover then reruns the whole proof, resampling every pad, g, r_p, every
  commitment's randomness, and the Σ mask. Reusing any pad after an abort
  would publish two maskings of one decoded value.
- **Digit sizing.** The digit count comes from Lemma `wo:lem:fold-overflow`.
  It is an `HonestFoldPolicySpec::OneShot { security_bits }` variant
  (`akita-params/src/sis/honest_fold_policy.rs:306-318`). It reuses the
  checked-integer tail computation of `fold_witness_linf_cap`
  (`sis/norm_bound.rs:416-437`), with the target replaced:
  - the logarithmic term is
    `ln_term = ⌈ln(2N) + (security_bits + ⌈log₂ L_cut⌉)·ln 2⌉`, where N is
    the fold's coefficient count over all groups and chunks and L_cut is the
    number of masked folds, so the union bound over folds stays within
    2^−128;
  - the cap is `t*` with `(L+1)²` on both balanced ends (§4.1, "both of its
    ends"), and no `min` with β_inf is taken unless β_inf is smaller (that
    case is the worst-case bound, with δ_fold = 0);
  - the digit count is `num_digits_for_linf_cap`
    (`sis/decomposition_digits.rs:140-156`).

  The planner applies the extra digits before
  `rounded_up_role_a_inf_norm` (`akita-planner/src/schedule_params/candidate.rs:157-187`,
  `planner.rs:126-158`). The A-role collision radius is therefore re-priced at
  the enlarged range (Lemma `wo:lem:randomized-base`). The schedule audit
  (`akita-schedules/src/audit.rs:318-350`) recomputes it from
  `num_digits_fold`, so the two cannot drift.
- **Challenge family.** Folds at or before the cut use the unfiltered family:
  `production_for_ring_dim`, d = 64, η² = 71. They have no operator-norm
  filter, so the lemma's 1/p factor is 1. ZK schedules reject an L2 route
  at or before the cut (§4.4), and with it the only filtered family that
  could appear there.

### 2.3 Acceptance

- **W2-a.** A ZK fold's transcript event log has no `Nonce` event at a
  `FoldResponse` site. A clear schedule's event log is unchanged (C0).
- **W2-b.** Forcing a too-small digit count through a test-only schedule
  makes `prove` return `FoldOverflow { level }` after exactly one probe, with
  no nonce absorbed. A rerun with a fresh RNG on a correct schedule
  verifies.
- **W2-c.** For the headline shapes, the planner's one-shot bound per fold
  is ≤ 2^−128/L_cut. A unit test checks it against the lemma's closed form in
  exact rational arithmetic. Perturbing `ln_term` down by one bit fails it.
- **W2-d.** The re-priced A ranks for folds before the cut equal Akita's at
  k = 2. Paper §6.2 states that the first two folds keep their ranks. If
  they differ, the test fails and the paper's claim is filed as an author
  question.

## 3. W1: outer hiding randomness per slice

### 3.1 What is sampled, and where it goes

| Commitment (made at) | Image hidden | Randomness | Joins | Alphabet |
|---|---|---|---|---|
| root, Jolt's packed polynomial (commit time) | each B slice `u_s` | `r_root,s` | w_1, beside the root's T digits | open basis of level 0 |
| D image at level j−1 (fold j's partial evaluations, `OpeningPayload`) | `v` (one image per level) | `r'_j` | w_j, the same fold's output | open basis of level j−1 |
| w_j at level j−1, j < k (`NextWitnessPayload`) | each B slice | `r_j,s` | w_{j+1}, beside the T digits of w_j | open basis of level j |
| w_k, the residual commitment (level k−1) | `u_a` (one slice) | `r_a` | **s**, not w_{k+1} (Def. `wo:def:residual`) | radix 16 (`jolt:specs/whiteout-sigma.md` §1.5) |

The table follows the paper's placement (§4.1, "Commitment randomness") and
the code's relation rows. Fold j's relation over w_{j+1} checks the B rows of
the commitment to w_j and fold j's own D rows
(`akita-params/src/layout/relation_rows.rs:139`). Each randomness vector
therefore sits in the witness whose relation already checks that image.

- **Count.** One owner computes it:
  `jolt_crypto::lattice::required_randomness_columns(image_coeffs, 0, d_b, log_basis, 8)`
  with `image_coeffs = n_b·d_b` per B slice (`n_d·d_d` per D image). The
  result is whole ring columns of the map's ring dimension. At d_b = 64,
  rank 1 and log_basis 3, that is 2,819 coefficients per slice, rounded up
  to 45 R_64 columns. The paper's 14 slices at k = 2 give the 32–40
  thousand entries it states (§6.2).
- **Sampling.** `sample_box_digits(rng, len, log_basis)`, constant time,
  drawn from the prover's CSPRNG and never from the transcript. The box
  {−b/2, …, b/2 − 1} has unit differences because b ≤ 64 < q^{1/4}/2
  (Lemma `wo:lem:stat-hiding`).
- **Alphabet and norms.** The randomness digits use the open-basis alphabet
  of the witness that holds them. That alphabet is the only one Akita's
  range check proves (`ring_switch.rs:123,198`; `stage1.rs:238-239`), so no
  new support constraint is needed, and the witness's ∞-norm `s_inf` does
  not change. The randomness affects norms only through length. The block
  count B and the coefficient count N of the next fold grow by the
  randomness span, which enters t* as √B and the one-shot `ln_term` as
  ln N. The planner sizes both on the widened layout. The B and D ranks are
  re-priced at the widened input width. The paper reports that all widened
  outer maps keep rank 1 at k = 2 (§6.2), and W1-d tests that claim.

### 3.2 Geometry and layout

- **Width authority.** `CommitmentSliceGeometry`
  (`akita-params/src/commitment_slicing.rs:202`) gains `randomness_columns`.
  `physical_input_width` becomes message width plus randomness width. Every
  equality check against B's `input_width` already reads
  `physical_input_width`, so they all inherit it:
  `layout/params/commitment.rs:107`, `precommitted.rs:417`, `builder.rs:112`,
  `akita-schedules/src/audit.rs:295`, `setup_contribution/plan/prepare.rs:444`,
  `schedule/profiles.rs:175`, `setup_envelope.rs:540`,
  `setup_prefix_slots.rs:518`, `akita-planner/src/schedule_params/candidate.rs:87`,
  and `relation_weights/compiler.rs:416`.
  `commitment_polynomial_count` (`commitment.rs:133`) divides by the
  message width, not by `physical_input_width`.
- **D's width.** `OpenCommitMatrixParams` gains the same field. Its input
  width becomes message plus randomness.
- **Matrix views.** Widening re-indexes every row after row 0
  (`flat_matrix.rs:94-106`), so the B and D views are cut at the widened
  width. Setup sizing (`setup_envelope.rs:44,84,115`,
  `setup_projection.rs:37`) reads the widened width, and the ZK setup is
  sized for the ZK schedule. A zero width leaves every view byte-identical.
  That matches `jolt-crypto`'s `AjtaiLevel { randomness_columns: 0 }`.
- **Witness layout.** `WitnessLayout` (`akita-params/src/witness.rs:97`)
  gains a tail section, `randomness`, placed after the compression spans so
  that every existing offset keeps its value. It holds the B randomness per
  group (relation order, slice-major), then the D randomness. Its length is
  0 in clear schedules. Both `tail::resolve` sinks,
  `grouped_witness_body_coefficients`, `chunk_shape`,
  `validate_internal_ranges`, and the classification `debug_assert` in
  `finalize.rs:287-332` count it. The section lies inside
  `live_coeff_len`, so the existing range check covers it.

### 3.3 Prover and verifier

- **Prover.**
  - `commit_outer_slices` (`arithmetic/commitment_stage.rs:88`) appends slice
    s's randomness to that slice's input.
  - `compute_relation_d_rows` (`d_rows.rs:31`) appends r'.
  - The randomness is kept per group in `RingRelationGroupWitness` and per
    level in `RingRelationWitness`. `emit_witness_tail` writes it.
  - The quotient recomputation (`relation_quotient.rs:625-740`) re-applies
    the widened B and D.
  - The root commitment's randomness lives in the prover's opening hint. A
    new `AkitaCommitmentScheme::commit_hiding(…, rng)` returns it there.
    That is the entry point W6's `HidingCommitment` calls
    (`jolt:specs/whiteout-interfaces.md` §3).
- **Relation weights.** B' and D' are setup columns with no gadget or
  structured term. `scatter_et` adds them on the prover side, and the
  verifier's `SetupContributionPlan` (`physical_b.rs`, `prepare.rs:61-106`,
  `segments.rs`) adds them on the other. Both read the one widened view, so
  T and R' take column offsets of the same view.
- **Kernels.** The randomness is secret, so it never passes through a
  digit-indexed LUT (W4 §1.3). Akita's B kernel runs on t̂, and t̂ is already
  secret in ZK mode. RP4 does not make the Akita commitment kernels constant
  time (A24).

### 3.4 Clear mode and acceptance

- **W1-a.** With `randomness_columns = 0` everywhere, C0 holds.
  `commit` and `commit_hiding` agree byte for byte when the randomness
  has length 0.
- **W1-b.** A ZK proof with randomness verifies. Two `commit_hiding` calls on
  one polynomial give different images, and each image passes a χ²
  uniformity test on its low bytes.
- **W1-c.** Flipping one randomness digit in w_{j+1} breaks fold j+1's
  relation (tamper test on the prover's witness through a test-only path).
- **W1-d.** For every ZK schedule, the planner's recomputed B/D ranks and
  `required_randomness_columns` agree with the schedule. A frozen-shape test
  pins the headline (rank 1 at folds 1–2).

## 4. W3: the masked folds and the cut

### 4.1 The cut: k = 2

The decision is k = 2, that is, the cut sits after Akita level 1's Stage 2.
Every condition of Def. `wo:def:source-cutoff` holds and is checked in
release by `ZkSchedule::validate`:

- **Folds ≤ k work mod q and check no norm in the clear.** No L2 route
  (`InnerCommitSecurityRoute::Linf`) is allowed at level ≤ k−1. The planner
  admits L2 only at level ≥ 3 (`akita-schedules/src/candidate.rs:34-46`),
  and ZK planning additionally forbids it at or before the cut.
- **Fold k passes no setup claim.** Level k−1 has no Stage 3. Level 0 may
  offload to level 1, and the paper's headline does so: level 1 then carries
  the setup group as a public segment of w_2.
- **Fold k is not the last fold before the terminal.** Level k−1's successor
  plan is the residual commitment, never `NextWitnessPlan::TerminalT`.

The planner chooses k = 2 for three reasons. It is the paper's headline. It
changes no matrix of the earlier folds (§6.2). And it masks the fewest folds,
so the pad schedule and the masked transcript are smallest. k ∈ {3, 4}
needs the rank-6 inner matrices of the third and fourth folds. The code is
generic over `cut_level`, but the catalog ships k = 2 only.

**ZK schedule.** A ZK schedule is
`ZkOpeningSchedule { prefix, cut: ResidualCutParams }`. `prefix` holds the
root plus `recursive_folds[..cut_level]`, with no terminal. `ResidualCutParams`
fixes the residual commitment for the commitment to w_k:

- inner A_a and outer [B_a | B′_a], both 3 × R_256, so d_A = d_B (A7);
- one slice;
- `r_a` columns from `required_randomness_columns(768, 0, 256, 4, 8)`, that is
  97 R_256;
- the one-shot fold policy.

Its matrices bind at the Gaussian radius (§4.1, "The residual commitment").
The planner prices them with Akita's SIS tables, which stay in Akita under
rule 3.

### 4.2 Transcript order from root to cut

The verifier for levels 0..k−1 runs today's `verify_root` and
`prepare_fold_replay` + `verify_fold`. It runs over a **masked channel**
(J1, J2) and with `OneShot` folds.

| Message (code site) | Clear today | ZK |
|---|---|---|
| `RootCommitment`, `RootPoint` | `public_all` | unchanged: hiding commitment, public point |
| `Openings` level 0, the statement claims | `public_all` | `bind_derived_all` (J3). In Jolt these are derived from Jolt's masked slots, so the verifier receives them as affine forms (§4.6) |
| `OpeningPayload` (D image or its compressed form, `root.rs:85-99`, `suffix.rs:200-214`) | `exchange_private_base` | **`exchange_all`, public**: a commitment hidden by r′ (W1). Clear bytes are identical, because clear `exchange_private_base` is `exchange_all` (`jolt:jolt-transcript/src/private.rs:73-115`) |
| fold seed and challenges | nonce + fork | fork(seed, 0) (W2) |
| `NextWitnessPayload` (B image of w_{j+1}; at level k−1 this is u_a) | `receive_n`, public | public: hidden by r (W1) |
| ring switch α, τ₀, τ₁ | public challenges | unchanged |
| stage-1 rounds, `stage1_range_image` | `exchange_private` | masked |
| stage-2 rounds, `stage2_w_eval` | `exchange_private` | masked |
| Stage 3: `stage3_claim`, rounds, `stage3_prefix_eval` (level 0 only) | `exchange_private` | **public**, verified in the clear (§4.3) |
| level ≥ 1 scalar openings (predecessor `w_eval`, setup prefix eval; `suffix.rs:174-177`, `coefficient_packing.rs:124-127`) | `public_all` | `bind_derived_all` (J3): the verifier holds them as affine forms |

The table assumes the following, which the Jolt fp128 configuration
satisfies. E = F (`akita-config/src/proof_optimized/fp128.rs:126-178`), so
there are no EOR messages. The open basis is b = 8 (`log_basis: 3`), so
stage 1 has no product stages (`digit_range.rs:170-178`). The route is Linf,
so there are no L2 messages before the cut.

### 4.3 Answers to the open questions (interfaces, "Open questions")

- **Extension-field products.** Jolt has none. For the fp128 presets E = F,
  so every masked claim is one F element. The only masked × masked products
  are listed in §4.4. A ZK schedule over an extension field
  (`E::DEGREE > 1`) is rejected by `ZkSchedule::validate` with
  `AkitaError::Unsupported`. The base-row expansion is specified below so a
  future caller does not reinvent it, but it is not built: it has no
  production caller, so building it now would add API without a caller.
  The future rule is as follows. An extension affine check `Σ a_i x_i + b = 0`,
  with public `a_i ∈ E` and masked `x_i ∈ E`, becomes `E::DEGREE` base rows.
  Row t is coordinate t of `Σ M(a_i) x⃗_i + b⃗`, where `M(a)` is the public
  multiplication matrix of a in the `ExtField` basis and `x⃗_i` are the
  per-coefficient pads (`MaskedVerifier::exchange_private`, `pads.rs`). A
  product x·y = z with x masked becomes `E::DEGREE²` single-operand product
  rows x_s·(M(β_s)y)_t = aux_{s,t}, with auxiliary slots, plus `E::DEGREE`
  affine rows z_t = Σ_s aux_{s,t}. This follows A1 (no sum-of-products
  rows).
- **Cut point of `response_l2_sq`.** After the cut. The value is sent only on
  an L2-route fold (`akita-types/src/l2_sites.rs:26-46`), and the ZK schedule
  forbids L2 at levels ≤ k−1. The value therefore never needs a field
  encoding. The remaining folds use L2, but they are replaced by the
  response proof.
- **Stage-3 setup-offloading claims.** Public. The setup-product claim
  σ_S = Σ_p S(p)·ω(p) depends only on the public setup and on τ₁, r_x and α,
  which are public challenges. The non-deferred verifier computes it from
  exactly those inputs (`akita-verifier/src/stages/stage2.rs:270-272`). The
  prefix evaluation is the MLE of a public slot (`stage3.rs:232-297`). The
  rounds are computed from the same data, so the whole Stage 3 runs on the
  clear channel even in ZK mode, and the verifier checks it directly. This
  also removes the only masked × masked product in a fold, `w·s` in the
  stage-2 relation weight (`akita-verifier/src/relation/evaluation.rs:90`):
  with s public, it is `w × public`. The M3b routing becomes `exchange_all`
  on both sides, and clear bytes are identical (same reason as
  `OpeningPayload`).
- **`NextWitnessInnerState` and scalar openings.** `NextWitnessInnerState` is
  the terminal's `t` (`prove/fold/mod.rs:546-576`). It is sent only by the
  last fold before the terminal, so it falls after the cut and a ZK proof
  never sends it. The scalar openings at levels ≥ 1 are the predecessor's
  masked `w_eval` and the public setup-prefix evaluation. The first is
  derived from a masked slot, so it uses `bind_derived_all` (J3), which in
  clear mode is one `public_all` and keeps the event log unchanged. A
  `public_all` of a decoded value in masked mode would also desynchronize
  the two transcripts, because the verifier holds only m̂. The setup-prefix
  evaluation is public and stays `public_all`.

### 4.4 Rows emitted per masked fold

There are three sources of rows:

- **Sumcheck rounds.** These go through `jolt_zk::masking::sumcheck_rows`,
  with `RoundFormat::Compressed` for stage 2 and the new
  `RoundFormat::EqFactored { equality_point }` (J4) for stage 1. Both formats
  omit the coefficient the round check defines, so they add no rows and
  return the running claim as an `AffineRow`.
- **Affine checks** become affine rows: the stage-2 input claim (batch ·
  range image + relation claim + opening claim), `trace_eval_target`, and
  the interstage batching.
- **Non-affine checks** go through the shared `ClaimDomain` lowering
  (`jolt:jolt-zk/src/masking/lowering.rs:26`). At b = 8 with the Linf
  route, a fold has exactly these:

| Check (site) | Clear formula | Lowering | Products | Aux slots |
|---|---|---|---|---|
| stage-1 leaf (`stage1.rs:186-190`) | out = Σ_k c_k·ri^k, deg 4 | factored: `scale·Π_{m<4}(ri − k_m(k_m+1))`, each factor a `MaskedOperand` with shifted constant `r̂i − k_m(k_m+1)` | 3 | 2 (the last product equals the running claim) |
| stage-2 virtual and binary terms (`stage2.rs:329`, `:457-475`) | w·(w+1) | one product x = w, y = w + 1, then affine | 1 | 1 |
| everything else in the stage-2 oracle | w × public, trace × public, s × w (s public) | affine | 0 | 0 |

That gives 4 product rows and 3 auxiliary slots per masked fold, 8 and 6 at
k = 2. The paper counts "two auxiliary values from the first two Akita folds"
(§6.1). The register files the difference as Q3; it does not change
soundness.

Finally, the cut row is `Witness(0) − (ŵ_eval − p) = 0`, at level k−1's
stage-2 point. `ResidualVar::Witness(0)` is claim 0 of the residual
commitment. Its point is level k−1's stage-2 point **r₂**, converted once
with `match_endianness` from Akita's `LOW_TO_HIGH` to the `HIGH_TO_LOW` order
of `EvaluationClaims` (A18; I4).

**One formula per check.** The leaf and stage-2 oracle formulas are written
once, generic over a domain with `ClaimDomain`'s five operations. The clear
verifier evaluates them in the value domain, and the masked verifier in
`RowsDomain`, `OperandShapeDomain` (layout), and `ValuesDomain` (prover
auxiliary values). `main` must not depend on `jolt-zk` yet: ZK reaches `main`
only when LZ chooses. A carried seam therefore adds an Akita trait
`akita_types::checks::CheckDomain` with the same five methods and a value
implementation. The `zk` module implements it by delegating to
`jolt_zk::masking::ClaimDomain`. The duplicated signature carries no logic,
and it disappears when ZK reaches `main` (U3). One rejected alternative
was clear and masked check functions side by side, which is a parallel
formula. The other was `main` depending on `jolt-zk` (and through it
`jolt-claims` and `jolt-r1cs`).

### 4.5 Masked verifier, prover, and `ResidualRows`

- **Channel generics.** `GrindingReplay::State` is fixed to
  `PrivateChannel<Error = TranscriptError>` over the concrete transcripts
  (`replay.rs:128-137`). It becomes generic: `ProverGrinding<'a, C>` and
  `VerifierGrinding<'a, 'p, C>`, with `C: Channel + PrivateChannel` and
  `C::Error: Into<AkitaError>`. The clear instantiation is the bare
  transcript. This is a carried seam (§8). Its only instantiation on `main`
  is the clear one, and the doc comment links akita#120.
- **Masked state.** In masked mode C is J2's prover channel over
  `&mut CommittedPads<F, O>`, or jolt-zk's `MaskedVerifier`
  (`jolt:jolt-zk/src/masking/pads.rs`). Jolt's prover owns the pads through
  stages 1–8 and lends them to Akita's opening for levels 0..k−1.
- **Verifier output.** The masked verifier never reads a hidden value. It
  records `MaskedFoldChecks`: per level, the masked sumcheck rounds with
  their `PadSlots`, the factored leaf and oracle inputs, and the affine
  checks, all in transcript order. It returns them with the public cut
  state.
- **`ResidualRows`.** `impl ResidualRows<F> for MaskedFoldChecks<F>`
  (`jolt:jolt-zk/src/masking/residual.rs`) lowers the recorded checks in
  `RowsDomain`. Its auxiliary slots come from the same `ClaimDomain` slot
  numbering, after Jolt's.
- **Totality.** Masked verification must accept arbitrary masked bytes.
  Jolt's `pad_layout` derives the schedule by replaying the masked
  verifiers over zero bytes with `CommittedPadsView::counting()`
  (`jolt:specs/whiteout-masking.md`, "Pad accounting"). The Akita prefix
  must therefore count the same pads on any input of the right shape. Public
  checks on zero commitments (decoding, length) pass, and no check reads a
  masked value.
- **Prover side.**
  - The fold prover keeps every clear self-check on decoded values.
  - It sends masked values through J2's channel. M3b's in-place warning
    (`ring_relation.rs:264-268`) holds: J2 sends m + p without writing it
    back.
  - It computes its auxiliary values in `ValuesDomain` through the same
    formulas. W6 sends them in the auxiliary block after Jolt's.

### 4.6 The hand-off to Jolt (W6) and the residual opening

In ZK mode the Akita root takes its statement claims as affine forms,
`MaskedOpeningClaims<F> = Vec<AffineRow<F>>`, which W6 builds from Jolt's
masked stage-8 claims. It does not take field values. The Dory stage-8 row
(`jolt:crates/jolt-verifier/src/verifier.rs:219-245`) has no Akita analogue:
the joint claim flows into level 0's affine checks.

At the cut, the prover returns
`ResidualOpening { w_k, t_hat_a, r_a, points, public_segments, blocks }`.
That is the `Witness` of W4 §4. Its fields:

- `w_k`: the digits of the witness at level k−1, `i64` in Akita's flat order;
- `t_hat_a`: the inner-image digits of the residual commitment;
- `r_a`: the residual randomness;
- `points`: one point (r₂, `HIGH_TO_LOW`);
- `public_segments`: whole R_256 elements of `w_k`. These are the setup-group
  response and the fully public inner digits (A3). In the ZK layout they are
  aligned to 256-coefficient boundaries, padding with zero cells.
- `blocks`: the `jolt_zk::whiteout::ResidualBlock`s describing N's residual
  part. They are A_a per block with gadget t̂, and [B_a|B′_a] on (t̂, r_a),
  all cut from Akita's setup stream.

The verifier gets `u_a` (raw until W5's compression) and the same blocks.

**Native order.** Akita's flat coefficient order of w_k at d_a = 256 is ring
by ring, consecutive coefficients. That is W4's native order, so the
segments carry it unchanged. The equivalence test (W3-e) pins this.

### 4.7 Acceptance

- **W3-a.** Completeness on random witnesses at test shapes. The pads and the
  decoded transcript satisfy every row emitted for levels 0..k−1
  (`ResidualRelation::first_violation` is `None`).
- **W3-b.** Coverage. Every pad slot used by the Akita prefix has a nonzero
  coefficient in some row or is a sumcheck coefficient whose round is
  lowered. This mirrors jolt's structural check.
- **W3-c.** Soundness probe. Shifting any one decoded Akita message by +1 and
  recomputing the auxiliary values as a cheater would breaks at least one
  row. The probe runs over all masked sites of both levels (test-only).
- **W3-d.** Layout. `CommittedPadsView::counting()` over zero bytes counts the
  same pads and auxiliary slots as an honest run. Changing any masked value
  leaves the count unchanged.
- **W3-e.** Residual equivalence.
  - `ResidualOpening` decodes to an s with `N·s = t`, under jolt-zk's
    `SigmaSetup`, for Akita's own `u_a`.
  - A dense evaluation of `w̃_k(r₂)` over the segments equals the
    decoded `w_eval`. This pins A4/A18 (MLE order, padding, stride).
  - Akita's commitment kernels agree with `jolt_crypto::lattice::TwoLevelLayout`
    semantics on s.
- **W3-f.** Routing. In clear mode, the event logs and bytes are unchanged
  after `OpeningPayload` and Stage 3 move to `exchange_all` and the scalar
  openings move to `bind_derived_all` (C0).
- **W3-g.** The validator rejects an L2 route at or before the cut, Stage 3
  at level k−1, a `TerminalT` successor at the cut, `E::DEGREE > 1`, and
  d_A ≠ d_B in the residual.

## 5. W5: `ResponseProver` for Akita

### 5.1 Ownership

jolt-zk owns the statement: s, N, Q_μ, L, the boxes, and e's logical
segments (W4 §3.1). Akita owns how the statement is proved. That covers
committing to e, the product reduction, the row batch, the range reduction,
the fused sumcheck, structured weights, setup offloading, compression with
its ξ digits, and the final opening. The implementing type is
`akita_pcs::zk::AkitaResponseProver<Cfg>`, which holds the response
schedule and the prover/verifier setup and implements
`ResponseProver<F, 256>`.

### 5.2 Physical e and the transcript

Physical e is jolt's `ResponseWitness.digits`, in `ResponseLayout` order,
followed by ξ_a, ξ_p, ξ_D (§5.4), with zero padding to 2^ν. Every digit lies
in the balanced radix-16 alphabet [−8, 7]. The ξ digits lie in {−1, 0}, a
subset of that alphabet, with the binary constraint added on their range
`I_bin`. The ordering of the response proof follows §4.3 of the paper:

1. Commit to e, an ordinary Akita commitment without randomness (e is
   simulatable; paper §5.3, last paragraph), under the response schedule.
2. Sample α. Send `t_×` and run the degree-2 product sumcheck over a = ⌈log₂ n_×⌉
   variables, then send `u_×` and `w_×` (2a + 3 field elements; these are
   public, the response proof is not ZK).
3. Sample η and form `A_j` and `b_η` from Items 2, 3 and 5, the three product
   claims, and the padding rows.
4. Run Akita's digit-range reduction on e at b = 16. This is the stage-1
   product tree with arities [2] then the leaf (`digit_range.rs:170-178`),
   reused as a standalone reduction. It ends with
   s_rng = Σ eq(τ, j)·e_j(e_j + 1).
5. Sample γ and ζ_bin, then run the degree-3 fused sumcheck
   `b_η + γ s_rng = Σ_x [A E + B E(E+1)]`. Send v and check
   `A(r)v + B(r)v(v+1)`.
6. Open E(r) = v with `batched_prove` under the response schedule: its root,
   its recursive folds, and its terminal, with ordinary clear grinding.

The new sumcheck kernels (product, fused) implement `jolt-sumcheck`'s prover
trait (parent spec, "Deferred tickets"), and the new messages use spongefish
codecs. The response schedule is a dense, bounded-digit family at log_basis 4
(the `DenseBounded` shape) with recursive setup planning. The planner sizes
it for |e| ≈ 2^26.

### 5.3 Structured weights and setup offloading

The verifier never builds `H`.

- **Local weights.** `A(r)` is evaluated from typed descriptions read
  through `ResponseStatement`'s accessors:
  - slot selections, digit radix powers, and eq weights of `Witness(j)`
    claims, which come from `LiftedRelation`'s symbolic claims;
  - negacyclic convolution by c, by the matrix entries, and by `L`.
  The local part is everything except N's and A_D's matrix entries. Those
  come from the setup.
- **Setup part.** That part is σ_N = Σ_p S(p)·ω_N(p). Every one of A_a,
  [B_a|B′_a], A_p together with L, and A_D is an offset-0 prefix view of
  the one setup stream (W4 §1.1). So ω_N is a sum of view weights over the
  same S, as in Akita's Stage 3 (`eq:sigma-s`).
- **One Stage 3.** σ_N is sent as a public claim. A fresh challenge λ
  batches it with the response opening root's own setup claim into that
  root's Stage 3: claim σ_root + λ·σ_N, weight ω_root + λ·ω_N. That is
  paper §4.3: one offload for both sets of matrices, about 1.2 KB.
  `active_setup_projection_geometry` (`setup_prefix_slots.rs:418-466`) gains
  a `ResponseRelation` projection, so the setup prefix and its precommitted
  slot cover the residual and pad matrices. The setup is sized for the ZK
  schedule.

### 5.4 Compression of u_a, t_p and t_D

**Cap.** Akita's chain accepts sources of at most 8 KiB, which is 512 Fp128
coefficients (`akita-params/src/compression.rs:19`). t_D has 64
coefficients and compresses today. u_a and t_p have 768 each.

**Decision.** `CompressionChainPlan::for_split_source` splits a source into
⌈len/512⌉ equal contiguous chunks and compresses each chunk with the
existing two-map chain, so a 768-coefficient image becomes 2 × 128 B.
Binding follows chunk by chunk from Lemma `wo:lem:compressed-binding`. Each
chunk's ξ digits join e, in chunk order. This costs 128 B more per large
image than a single chain would (Q4). A third map was rejected because it
changes `COMPRESSION_MAP_COUNT` for every clear schedule.

**Who sends what.**

- **u_a.** Akita compresses it at level k−1. `ResidualCutParams` gains
  `CommitmentPayloadMode::Compressed`, which until W5 is `Raw`: the ZK
  validator allows `Raw` only at the cut, and only until this PR. Akita
  appends ξ_a to e.
- **t_p and t_D.** jolt-zk sends these, so the compression must reach it
  without jolt depending on Akita (I1). J5 adds a compression hook to
  `ResponseProver`: `compress_image(&self, source, image) -> CompressedImage`
  and `compressed_len`. `commit_pads` and the attempt loop then send the
  compressed forms, and `ResponseStatement` exposes the raw images to Akita,
  which appends ξ_p and ξ_D. The oracle input for c becomes
  `(σ, cm(t_D), n)`, matching paper H2.

Item 2 of the statement thus becomes the compression equations of all three
images, plus `t_D = A_D G⁻¹(V)`. Every coordinate of V is a digit vector in
e that A_D maps to the raw t_D.

### 5.5 The last PR: ZK mode on, and the hidden-vs-transparent tests

The last PR does the following:

- publishes `ZkCommitmentConfig`, the ZK prove and verify entry points, and
  `AkitaResponseProver`;
- generates the `_zk` schedule family and its catalog rows;
- sets `ProtocolFeatureSet::zk`;
- adds the CI job `test-zk` (workspace tests with `--features zk`).

It adds the following tests. In each, the proof comes from an end-to-end
Akita-side harness: Jolt-independent masked claims on a synthetic statement,
a `jolt-zk` whiteout backend over Akita's `ResidualRows`, and
`AkitaResponseProver`.

| Test | Hidden (ZK) proof must | Transparent (clear) proof must | Why it discriminates |
|---|---|---|---|
| **HT1, public-site allowlist** | The verifier's event log puts every received byte in exactly one class: masked slot, randomized commitment, public challenge-derived (stage 3, points), Σ message, or response proof. No `exchange_private` reaches a clear channel. | have private messages on the clear channel (positive control) | A message that bypasses masking shows up as an unclassified or clear-private event. The test is deterministic. |
| **HT2, two-witness distinguisher** | Over N = 200 proofs each for two witnesses with equal public input and shape, a per-site χ² homogeneity test on the low 16 bits of every masked slot, commitment image and `t_D`, Bonferroni-corrected, finds no difference. | fail the same test at the same N on clear proofs of the same two witnesses (positive control) | It is the same distinguisher on both. It is calibrated by the control, so a too-weak test cannot pass vacuously. |
| **HT3, fresh randomness** | Two ZK proofs of one witness with different RNG seeds share no masked value, commitment image or nonce. Two clear proofs are byte-identical. | — | It catches pad or randomness reuse, including across a `FoldOverflow` rerun (forced via a test schedule). |
| **HT4, tamper** | reject a flip in each masked slot, commitment, u_a, t_p, t_D, n, and each response-proof message | — | It covers soundness of the masked path end to end. |
| **HT5, clear invariance** | — | match the merge-base proof bytes for every existing catalog row (C0) | It is the extension contract. |

HT2 is statistical evidence, not a proof of zero knowledge. It and HT1 are
the "hidden-vs-transparent" gate of RP4. HT2 runs in CI's `test-zk` job with a fixed seed, so it is deterministic.

### 5.6 Acceptance (W5 PRs before the last)

- **W5-a.** `AkitaResponseProver` proves and verifies every statement that
  jolt's `CheckingResponseProver` accepts. On test shapes, with tampered
  witnesses, it rejects every statement that `check_witness` rejects (items
  1–5 one at a time).
- **W5-b.** The verifier's `A(r)` and `B(r)` equal a dense contraction of `H`
  on small shapes. That contraction is a test oracle built from the
  statement's definition, not a second formula: it applies `H` row by row
  through `check_witness`'s own maps.
- **W5-c.** The combined Stage 3 verifies, and a tampered σ_N is rejected.
- **W5-d.** Split compression round-trips. A one-digit flip in any ξ chunk is
  rejected.
- **W5-e.** Size and time are reported against the paper's model: response
  proof bytes, the +1.2 KB offload, and prover seconds at the headline.

## 6. M6e: what is deleted

M6e deletes:

- **`akita-zk-prover` and `akita-zk-verifier`, entirely.** That covers their
  `Cargo.toml`, sources and tests, the workspace members (root
  `Cargo.toml:20-21`), the two `check-crate-deps.sh` calls
  (`.github/workflows/ci.yml:86-87`), and their `Cargo.lock` entries. Both
  crates' only contents moved byte for byte into jolt-zk (M6d):
  `sample_zero_constant_term` became `jolt:jolt-zk/src/whiteout/sample.rs`,
  and `LiftedFunctional` became `whiteout/ring.rs`, with a typed error. They
  have no dependents, and no Akita-specific item remains.
- **The M1c/M5c re-export shims** (`akita_algebra::ring`, `akita_challenges`
  paths), once the pinned jolt has ported `jolt-akita` to jolt paths in RP3
  (parent spec, "The add-first rule also runs the other way").

Nothing stays. The future Akita ZK code lives in new `zk` modules of the
protocol crates, not in separate `akita-zk-*` crates. A crate per role would
recreate the verifier/prover split that the protocol crates already have.

Acceptance: the workspace builds with and without `zk`, and
`cargo metadata` lists no `akita-zk-*` package. That is all.

## 7. What jolt must provide

### Before RP4 (J1–J5, at RP4's jolt pin)

| Id | Change | Needed by |
|---|---|---|
| J1 | `ProverRoundContext`/`VerifierRoundContext` expose `type Channel: Channel + PrivateChannel` and `channel_mut()` in place of the concrete `transcript_mut()`. Clear bytes unchanged. | W3: masked Akita sumchecks through `prove_single`/`verify_single` |
| J2 | A prover-side masked `PrivateChannel` over `&mut CommittedPads<P, O>` and a transcript: per-coefficient pads for `E: ExtField<F>`. A release check that the message field is the pad field (typed `ZkError`) is needed because the trait's methods are generic over F. | W3 prover |
| J3 | `PrivateChannel::bind_derived_all` and `bind_derived_base_all`. Clear: one `public_all`. Masked: nothing. | W3 routing (§4.2) |
| J4 | `masking::RoundFormat::EqFactored { equality_point }` in `sumcheck_rows`, using `EqFactoredRound`'s weights. | W3 stage 1 |
| J5 | The `ResponseProver` compression hook (§5.4), with `ResponseStatement` exposing raw images. Also `ZkError::FoldOverflow`, which has no producer until W6 but is part of the trait surface Akita returns through. | W5 |

These are additive jolt changes to crates Akita already pins. If RP3 is still
open, they join its second batch. Otherwise a small jolt review point is
needed before W3 and W5 (U1).

### W6 (RP5) needs from RP4

- **Akita entry points.**
  - `ZkCommitmentConfig` and `commit_hiding` (W1), behind `HidingCommitment`.
  - A ZK opening prover that takes `&mut CommittedPads` and
    `MaskedOpeningClaims`, and returns `ResidualOpening` (W3).
  - A masked verifier that returns `MaskedFoldChecks: ResidualRows` and the
    cut state (W3).
  - `AkitaResponseProver` (W5).
- **Layout.** A counting-mode entry for the Akita prefix, so jolt's
  `pad_layout` counts Jolt and Akita pads and auxiliary slots in one
  derivation (§4.5 totality).
- **Transcript order** of a ZK Jolt + Akita proof:
  1. preamble and hiding commitment;
  2. `t_p` (compressed);
  3. Jolt stages 1–7 masked;
  4. stage-8 claims handed to Akita;
  5. Akita levels 0..k−1 masked;
  6. the auxiliary block: Jolt's slots, then Akita's;
  7. ρ, σ, cm(t_D), n;
  8. the response proof.
- **Errors.** `AkitaError::FoldOverflow` → `ZkError::FoldOverflow`. The jolt
  prover returns it without retrying. The caller reruns the whole proof,
  since the probability is ≤ 2^−128 by construction and a loop would be an
  unexercised path.
- **Cleanups.** W4's raw `t_p`/`t_D` path is deleted once J5's hook is wired.

## 8. PR slicing for RP4

`dev` needs passing CI and a review from another `dev` contributor. The
extension contract adds a constraint. A PR that changes `main` files beyond
seam registrations carries a seam: the seam is opened against `main` in
parallel and squash-merged into `dev` first (branches.md, "Carry a seam").
The seams below are behavior-preserving on `main`.

| # | PR | Carried seam(s) on `main` | Depends on | Test bar |
|---|---|---|---|---|
| 1 | **M6e**: delete `akita-zk-*` and the shims | — | jolt pin with RP3 | builds with and without `zk`; `cargo metadata` check; C0 |
| 2 | **W2**: `FoldResponseSearch::OneShot`, `FoldOverflow`, the `OneShot` honest-fold policy | S1: grinding-plan entry with `Counter` only; `HonestFoldPolicySpec` variant registration | 1 | W2-a…d; C0 |
| 3 | **W1**: randomness columns, randomness tail section, `commit_hiding` | S2: `randomness_columns` (= 0) in `CommitmentSliceGeometry` and `OpenCommitMatrixParams`; S3: zero-length randomness tail section | 1 | W1-a…d; C0 |
| 4 | **W3a**: routing corrections: `OpeningPayload` and Stage 3 to `exchange_all`, scalar openings to `bind_derived_all` | S4: this whole PR is a `main` fix-up of M3b | J3 | W3-f; C0 |
| 5 | **W3b**: channel-generic grinding; `CheckDomain` formulas | S5: `ProverGrinding<C>`/`VerifierGrinding<C>` and `CheckDomain` with the value implementation | J1, 4 | C0; leaf and oracle formulas unchanged on random inputs (the clear verifier suite) |
| 6 | **W3c**: `ZkOpeningSchedule`, the residual cut, the masked verifier and prover, `ResidualRows`, `ResidualOpening` (u_a raw) | S6: suffix loop takes `SuffixEnd::{Terminal, Cut}` | 2, 3, 5, J2, J4 | W3-a…e, W3-g |
| 7 | **W5a**: product and fused reductions, structured weights, `AkitaResponseProver` with raw images | — | 6, J5 | W5-a, W5-b |
| 8 | **W5b**: response-relation setup offloading | S7: `ResponseRelation` projection registration | 7 | W5-c |
| 9 | **W5c**: split compression; compressed u_a, t_p, t_D | S8: `for_split_source` (unused by clear plans) | 7, J5 | W5-d |
| 10 | **W5d (last)**: publish the config and entry points, generate ZK artifacts, `zk: true`, CI `test-zk` | — | 8, 9 | HT1–HT5, W5-e; the remaining W4 audit items (NEON/AVX NTT, `Fp128` encoding in the attempt fork), which are W4's last-PR gate |

Within the table, 2 and 3 are independent, and so are 7–9 once 7 lands. No
PR depends on an unmerged PR in jolt, because J1–J5 are in the pin. PRs
2–9 add no public selecting API, so ZK cannot be selected until PR 10.

## 9. Soundness and zero knowledge, mapped to the paper

| Choice | Paper item it discharges |
|---|---|
| No fold nonce at or before the cut; one probe; whole-proof rerun with all randomness fresh | §4.1 "Fold responses"; Lemma `wo:lem:fold-overflow`; H1 |
| One-shot digits at 2^−128 / L_cut; A re-priced at the enlarged range | Lemma `wo:lem:randomized-base` ("inner-matrix collision radii recomputed") |
| Randomness per slice, entropy ≥ image + 2λ + 8, open-basis alphabet, in the next witness | §4.1 "Commitment randomness"; Lemma `wo:lem:stat-hiding`; H9 |
| Randomness inside `live_coeff_len`, so it is range-checked | Lemma `wo:lem:randomized-base` ("specified digit alphabet ... included in its range check") |
| Residual matrices 3 × R_256 at the Gaussian radius, d_A = d_B | §4.1 "The residual commitment"; A7 |
| Stage 3 public, setup claim not passed by fold k | §3.1 "Messages computed from public data alone"; Def. `wo:def:source-cutoff` |
| Every masked check lowered once through `ClaimDomain`; rows never dropped | Lemma `wo:lem:residual-correct` |
| Cut at k = 2: no L2 route and no clear norm before the cut | Def. `wo:def:source-cutoff` |
| e committed without randomness; ordinary clear Akita opening | §4.3; §5.3 final paragraph |
| Split compression with ξ digits in e | Lemma `wo:lem:compressed-binding` per chunk |

## 10. Ambiguity register

The numbering continues `jolt:specs/whiteout-sigma.md` (A1–A23).

- **A24, constant-time Akita kernels.** Akita's inner, outer and D kernels
  run on secret digits (the witness, t̂, and the randomness) in ZK mode. The
  paper's zero-knowledge theorem excludes timing. W4 made the Σ attempt
  constant time because a rejected attempt's values must never be observed.
  The masked-prefix kernels do not have that property: they run once per
  proof on values that the proof itself hides. RP4 does not audit them.
  The note records this as a known side-channel posture and does not
  claim more.
- **A25, Jolt's catalog levels.** The survey did not decode the `.aks`
  artifacts. Which levels of the current Jolt fp128 catalog use the L2 route,
  and whether level 0 offloads, is unverified. The ZK planner produces its
  own schedules, so this affects only the paper's comparison.
- **A26, small test shapes.** It is unverified that the ZK planner finds
  feasible schedules with root, one recursive fold and the residual cut at
  test sizes (T ≤ 2^16). If it does not, the tests need a test-only schedule
  constructor, which W3c adds.
- **A27, setup-group alignment.** Aligning the setup-group response to
  R_256 boundaries (§4.6) adds zero cells to w_k. The count is unmeasured.
- **A28, residual slicing.** This note fixes one slice for u_a, matching
  W4's 97-column r_a. Whether slicing the residual helps the verifier is not
  explored.
- **A2, A3, A12 (from W4)** are resolved here: §4.3, §4.6, and §5.4.

## 11. Questions

### For the paper authors

- **Q1, pads instead of randomness for j < k.** A compressed commitment made
  before fold k never has its image revealed: its ξ digits stay inside
  witnesses that are masked or Gaussian-hidden. Could the compressed form be
  masked with 8 pads, instead of hiding the image with 2.8k randomness
  coefficients per slice? This note follows the paper (randomness), but the
  answer could cut W1's footprint. The paper's argument for hiding the full
  image ("the proof of the response ... contains the binary decompositions")
  covers only the two residual-relation commitments.
- **Q2, A7/A8/A9/A14 carried from W4.** Still open: d_A = d_B, L, the box
  envelope, and returning ⊥ after T_att.
- **Q3, auxiliary count.** §6.1 counts two auxiliary values for the first two
  Akita folds. The code's checks need 3 auxiliary slots and 4 products per
  fold (§4.4). Is the paper using a different leaf form, or undercounting?
- **Q4, compression of images above 8 KiB.** The cost script's
  `FCHAIN_MAX_IMAGE = 884` disagrees with Akita's 512-coefficient cap. The
  split chain (§5.4) costs 128 B per large image. Is that what the model
  should count?
- **Q5, union bound across folds.** Lemma `wo:lem:fold-overflow` is applied
  per fold with "δ_fold the sum of the bounds". This note sizes each fold at
  2^−128/L_cut. Confirm that the paper's 2^−950 figure is per fold and not
  the sum.

### For the user

- **U1.** J1–J5 are jolt changes that RP4 needs at its pin. They join RP3's
  second batch if it is still open; otherwise they need a jolt review point
  before RP4's W3 and W5 PRs.
- **U2.** RP4 carries eight seams onto `dev` (S1–S8). Each also needs a
  `main` PR reviewed by LZ, which RP4 does not wait for. Confirm that this
  matches the plan for the Akita maintainers.
- **U3.** `CheckDomain` duplicates `jolt_zk::masking::ClaimDomain`'s
  signature until ZK reaches `main` (an I2 exception with a removal
  condition). The alternative is a `jolt-zk` dependency on Akita `main`.

## Not verified by this note

- The survey of Akita code is from reading at `87b4ca046`. No branch of this
  design was built or run.
- The headline numbers are from the paper and the W4 note: 45 R_64 columns
  per slice, 97 R_256 for r_a, 4 products per fold, and the +1.2 KB offload.
  The ZK planner has not been run.
- Byte identity of clear `exchange_private_base` with `exchange_all` comes
  from reading `clear_private_channel!`. W3-f tests it.
- Whether the planner's rank re-pricing reproduces the paper's "rank 1, ranks
  unchanged at k = 2" is a test (W1-d, W2-d), not a fact here.
