# Existing planner: setup-offload boundary survey

Baseline: `fddf785cf1d6011b508c271d78e69958d1fde4af`. Existing recursive comparator and `RootAndFirstChildV1`; zero offloads remain eligible. No automatic root threshold is applied.

Each cell opens one polynomial in one group, with the stock dense or one-hot source contract. W is the witness chunk count and R the number of leading levels using it. W1R0 is unchunked. One-hot source density is the stock config contract and is separate from W. Custom field/chunk combinations use the stock field config with only its witness-chunk policy changed; they are not claims that application presets already ship for those combinations.

Every newly generated successful cell is a complete planner result independently audited by `ValidatedScheduleCatalog::try_new` and saved as a canonical `.aks` artifact. Committed scalar rows from the baseline catalogs are reused where available and marked committed_catalog in the records/TSV; their unavailable timing and size estimates are left blank. Their four source catalogs are frozen byte-for-byte in baseline_catalogs/, so retiring runtime companion artifacts does not alter this evidence; each reused record retains its original baseline artifact path. Newly generated sizes are planner estimates, not end-to-end measured proof sizes or verifier timings. Planning durations are diagnostics collected under concurrent system load and are not controlled performance benchmarks.

Recorded cells: **584** across **60/60** field/source/layout combinations; statuses: ok=561, timeout=23.

Count coverage: **60/60** configurations have both zero and positive offload observations; **46/60** have a two-offload observation. Adjacent 0→positive changes are observed in **60/60** configurations, and adjacent 1→2 changes in **2/60**. These are local observations, not global monotonicity claims.

The table lists tested variable counts only. A range such as 20–24 means every integer in that range was successfully planned. Separate points such as 20, 24 do not establish the behavior at 21–23. Missing cells are not interpolated. A dash in a count column means that count was not observed in a completed cell; it does not establish impossibility. An error or timeout does not establish an offload count, and the largest tested point does not establish behavior at every larger input.

| Field | Source | Layout | 0 offloads: tested nv | 1 offload: tested nv | 2 offloads: tested nv | Errors: nv | Timeouts: nv | Missing initial-grid nv |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fp32 | dense | W1R0 | 16–22 | 23–32, 40 | 44, 48 | — | — | — |
| fp32 | dense | W2R1 | 16, 20, 22 | 23–24, 32, 36 | 40 | — | — | — |
| fp32 | dense | W2R2 | 16, 24 | 25–26, 28, 32, 36 | 40 | — | — | — |
| fp32 | dense | W2R3 | 16, 20, 22–23 | 24, 32 | — | — | 40 | — |
| fp32 | dense | W4R1 | 16, 24 | 25–26, 28, 32, 36 | 40 | — | — | — |
| fp32 | dense | W4R2 | 16, 24, 26 | 27–28, 32 | 36, 40 | — | — | — |
| fp32 | dense | W4R3 | 16, 24, 26 | 27–28, 32 | 36, 40 | — | — | — |
| fp32 | dense | W8R1 | 16, 19, 21 | 22, 24, 32, 36 | 40 | — | 20 | — |
| fp32 | dense | W8R2 | 16, 24, 27 | 28, 30, 32 | 36, 40, 48 | — | — | — |
| fp32 | dense | W8R3 | 16, 24, 26 | 27–28, 32, 36 | 40 | — | — | — |
| fp32 | onehot | W1R0 | 16–24 | 25–32, 39–40 | 44, 48 | — | — | — |
| fp32 | onehot | W2R1 | 16, 24 | 25–26, 28, 32, 40 | — | — | — | — |
| fp32 | onehot | W2R2 | 16, 24 | 25–26, 28, 32, 36 | 40, 42, 48 | — | — | — |
| fp32 | onehot | W2R3 | 16, 24, 26 | 27–28, 32, 36 | 40, 42, 48 | — | — | — |
| fp32 | onehot | W4R1 | 16, 24 | 25–26, 28, 32, 40 | — | — | — | — |
| fp32 | onehot | W4R2 | 16, 24, 26 | 27–28, 32, 36 | 40, 42, 48 | — | — | — |
| fp32 | onehot | W4R3 | 16, 24, 28 | 29–30, 32 | 40, 48 | — | — | — |
| fp32 | onehot | W8R1 | 16, 24, 26 | 27–28, 32, 40 | — | — | — | — |
| fp32 | onehot | W8R2 | 16, 24, 28, 30 | 31–32, 36 | 40, 42, 48 | — | — | — |
| fp32 | onehot | W8R3 | 16, 24, 28, 30 | 31–32 | 40, 48 | — | — | — |
| fp64 | dense | W1R0 | 16–21 | 22–32, 36 | 38 | — | 40, 48 | — |
| fp64 | dense | W2R1 | 16, 20–21 | 22, 24, 32 | — | — | 40 | — |
| fp64 | dense | W2R2 | 16, 24–25 | 26, 28, 32 | — | — | 40 | — |
| fp64 | dense | W2R3 | 16, 24–25 | 26, 28, 32 | — | — | 40 | — |
| fp64 | dense | W4R1 | 16, 20–21 | 22, 24, 32 | — | — | 40 | — |
| fp64 | dense | W4R2 | 16, 24–25 | 26, 28, 32 | — | — | 40 | — |
| fp64 | dense | W4R3 | 16, 24–25 | 26, 28 | — | — | 32, 40 | — |
| fp64 | dense | W8R1 | 16, 20, 22–23 | 24, 32 | — | — | 40 | — |
| fp64 | dense | W8R2 | 16, 24–25 | 26, 28 | 32 | — | 40, 48 | — |
| fp64 | dense | W8R3 | 24–25 | 26, 28, 32 | — | — | 16, 40 | — |
| fp64 | onehot | W1R0 | 16–24 | 25–32, 39 | 40, 48 | — | — | — |
| fp64 | onehot | W2R1 | 16, 24 | 25–26, 28, 32 | 40 | — | — | — |
| fp64 | onehot | W2R2 | 16, 24, 28 | 29–30, 32, 36 | 40, 42, 48 | — | — | — |
| fp64 | onehot | W2R3 | 16, 24, 28 | 29–30, 32 | 40 | — | 48 | — |
| fp64 | onehot | W4R1 | 16, 24 | 25–26, 28, 32 | 40 | — | — | — |
| fp64 | onehot | W4R2 | 16, 24, 28 | 29–30, 32, 36 | 40, 42, 48 | — | — | — |
| fp64 | onehot | W4R3 | 16, 24, 28 | 29–30, 32 | 40, 48 | — | — | — |
| fp64 | onehot | W8R1 | 16, 24, 26 | 27–28, 32 | 40 | — | — | — |
| fp64 | onehot | W8R2 | 16, 24, 28 | 29–30, 32, 36 | 42, 48 | — | 40 | — |
| fp64 | onehot | W8R3 | 16, 24, 28, 30 | 31–32 | 40, 48 | — | — | — |
| fp128 | dense | W1R0 | 16–24 | 25–32 | 36, 40 | — | 48 | — |
| fp128 | dense | W2R1 | 16, 24 | 25–26, 28, 32 | 36, 40 | — | — | — |
| fp128 | dense | W2R2 | 16, 24 | 25–26, 28, 32 | 36, 40 | — | — | — |
| fp128 | dense | W2R3 | 16, 24 | 25–26, 28, 32 | — | — | 40 | — |
| fp128 | dense | W4R1 | 16, 24 | 25–26, 28, 32 | 36, 40 | — | — | — |
| fp128 | dense | W4R2 | 16, 24, 28 | 29–30, 32 | 36, 40 | — | — | — |
| fp128 | dense | W4R3 | 16, 24, 28 | 29–30, 32, 36 | 40 | — | — | — |
| fp128 | dense | W8R1 | 16, 24 | 25–26, 28, 32 | — | — | 40 | — |
| fp128 | dense | W8R2 | 16, 24, 28 | 29–30 | 32 | — | 40, 48 | — |
| fp128 | dense | W8R3 | 16, 24, 28 | 29–30, 32, 36 | 40 | — | — | — |
| fp128 | onehot | W1R0 | 16–24, 26, 28 | 25, 27, 29–32, 36 | 40, 42, 48 | — | — | — |
| fp128 | onehot | W2R1 | 16, 24, 28 | 29–30, 32, 36 | 40 | — | — | — |
| fp128 | onehot | W2R2 | 16, 24, 28 | 29–30, 32, 36 | 40, 42, 48 | — | — | — |
| fp128 | onehot | W2R3 | 16, 24, 28 | 29–30, 32 | 39–40, 48 | — | — | — |
| fp128 | onehot | W4R1 | 16, 24, 28 | 29–30, 32, 36 | 40 | — | — | — |
| fp128 | onehot | W4R2 | 16, 24, 32 | 33, 35 | 39–40, 48 | — | — | — |
| fp128 | onehot | W4R3 | 16, 24, 32 | 33, 35, 39–40 | 44, 48 | — | — | — |
| fp128 | onehot | W8R1 | 16, 24, 28 | 29–30, 32, 36 | 40 | — | — | — |
| fp128 | onehot | W8R2 | 16–28 | 29–34 | 35–48 | — | — | — |
| fp128 | onehot | W8R3 | 16, 24, 28, 30 | 31–32, 40 | 44, 48 | — | — | — |

## Observed transition brackets

An arrow between neighboring integers confirms that local change. An ellipsis marks a bracket with unmeasured or unsuccessful interior points. These are observed changes within the tested range, not a guarantee about every larger input.

| Field | Source | Layout | Changes in selected offload count |
| --- | --- | --- | --- |
| fp32 | dense | W1R0 | nv 22→23: 0→1; nv 40…44: 1→2 |
| fp32 | dense | W2R1 | nv 22→23: 0→1; nv 36…40: 1→2 |
| fp32 | dense | W2R2 | nv 24→25: 0→1; nv 36…40: 1→2 |
| fp32 | dense | W2R3 | nv 23→24: 0→1 |
| fp32 | dense | W4R1 | nv 24→25: 0→1; nv 36…40: 1→2 |
| fp32 | dense | W4R2 | nv 26→27: 0→1; nv 32…36: 1→2 |
| fp32 | dense | W4R3 | nv 26→27: 0→1; nv 32…36: 1→2 |
| fp32 | dense | W8R1 | nv 21→22: 0→1; nv 36…40: 1→2 |
| fp32 | dense | W8R2 | nv 27→28: 0→1; nv 32…36: 1→2 |
| fp32 | dense | W8R3 | nv 26→27: 0→1; nv 36…40: 1→2 |
| fp32 | onehot | W1R0 | nv 24→25: 0→1; nv 40…44: 1→2 |
| fp32 | onehot | W2R1 | nv 24→25: 0→1 |
| fp32 | onehot | W2R2 | nv 24→25: 0→1; nv 36…40: 1→2 |
| fp32 | onehot | W2R3 | nv 26→27: 0→1; nv 36…40: 1→2 |
| fp32 | onehot | W4R1 | nv 24→25: 0→1 |
| fp32 | onehot | W4R2 | nv 26→27: 0→1; nv 36…40: 1→2 |
| fp32 | onehot | W4R3 | nv 28→29: 0→1; nv 32…40: 1→2 |
| fp32 | onehot | W8R1 | nv 26→27: 0→1 |
| fp32 | onehot | W8R2 | nv 30→31: 0→1; nv 36…40: 1→2 |
| fp32 | onehot | W8R3 | nv 30→31: 0→1; nv 32…40: 1→2 |
| fp64 | dense | W1R0 | nv 21→22: 0→1; nv 36…38: 1→2 |
| fp64 | dense | W2R1 | nv 21→22: 0→1 |
| fp64 | dense | W2R2 | nv 25→26: 0→1 |
| fp64 | dense | W2R3 | nv 25→26: 0→1 |
| fp64 | dense | W4R1 | nv 21→22: 0→1 |
| fp64 | dense | W4R2 | nv 25→26: 0→1 |
| fp64 | dense | W4R3 | nv 25→26: 0→1 |
| fp64 | dense | W8R1 | nv 23→24: 0→1 |
| fp64 | dense | W8R2 | nv 25→26: 0→1; nv 28…32: 1→2 |
| fp64 | dense | W8R3 | nv 25→26: 0→1 |
| fp64 | onehot | W1R0 | nv 24→25: 0→1; nv 39→40: 1→2 |
| fp64 | onehot | W2R1 | nv 24→25: 0→1; nv 32…40: 1→2 |
| fp64 | onehot | W2R2 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp64 | onehot | W2R3 | nv 28→29: 0→1; nv 32…40: 1→2 |
| fp64 | onehot | W4R1 | nv 24→25: 0→1; nv 32…40: 1→2 |
| fp64 | onehot | W4R2 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp64 | onehot | W4R3 | nv 28→29: 0→1; nv 32…40: 1→2 |
| fp64 | onehot | W8R1 | nv 26→27: 0→1; nv 32…40: 1→2 |
| fp64 | onehot | W8R2 | nv 28→29: 0→1; nv 36…42: 1→2 |
| fp64 | onehot | W8R3 | nv 30→31: 0→1; nv 32…40: 1→2 |
| fp128 | dense | W1R0 | nv 24→25: 0→1; nv 32…36: 1→2 |
| fp128 | dense | W2R1 | nv 24→25: 0→1; nv 32…36: 1→2 |
| fp128 | dense | W2R2 | nv 24→25: 0→1; nv 32…36: 1→2 |
| fp128 | dense | W2R3 | nv 24→25: 0→1 |
| fp128 | dense | W4R1 | nv 24→25: 0→1; nv 32…36: 1→2 |
| fp128 | dense | W4R2 | nv 28→29: 0→1; nv 32…36: 1→2 |
| fp128 | dense | W4R3 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | dense | W8R1 | nv 24→25: 0→1 |
| fp128 | dense | W8R2 | nv 28→29: 0→1; nv 30…32: 1→2 |
| fp128 | dense | W8R3 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W1R0 | nv 24→25: 0→1; nv 25→26: 1→0; nv 26→27: 0→1; nv 27→28: 1→0; nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W2R1 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W2R2 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W2R3 | nv 28→29: 0→1; nv 32…39: 1→2 |
| fp128 | onehot | W4R1 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W4R2 | nv 32→33: 0→1; nv 35…39: 1→2 |
| fp128 | onehot | W4R3 | nv 32→33: 0→1; nv 40…44: 1→2 |
| fp128 | onehot | W8R1 | nv 28→29: 0→1; nv 36…40: 1→2 |
| fp128 | onehot | W8R2 | nv 28→29: 0→1; nv 34→35: 1→2 |
| fp128 | onehot | W8R3 | nv 30→31: 0→1; nv 40…44: 1→2 |

## Observed decreases in offload count

A larger instance can select fewer offloads. These endpoints demonstrate that a single monotone cutoff is not an invariant of the existing comparator. Untested intermediate points remain unknown.

| Field | Source | Layout | Smaller nv: offloads | Larger nv: offloads |
| --- | --- | --- | --- | --- |
| fp128 | onehot | W1R0 | 25: 1 | 26: 0 |
| fp128 | onehot | W1R0 | 27: 1 | 28: 0 |

Full per-cell values and errors: [results.tsv](results.tsv). Canonical schedules: [schedules/](schedules/). Resumable commands, status, and diagnostics: [records/](records/).

## Interpretation

The existing comparator minimizes the padded setup envelope, then the first remaining direct-scan capacity, then the modeled proof/work score. Candidate geometry and power-of-two buckets can change the winner at neighboring sizes. Offload counts alone do not establish actual verifier latency. The adaptive refinement mode narrows observed changes only; equal counts at sampled endpoints can still hide untested islands, so the tables never interpolate them.

## Scope

This survey covers the unchunked layout and W2/W4/W8 at R1, R2, and R3 across all six field/source pairs. Alternative one-hot density contracts are outside this grid; the stock density is one nonzero per 256 source entries. Each row is a scalar opening; grouped openings can select a different number of offloads at the same final-group variable count. The previously observed two-offload grouped W8R2 row at final nv34 is therefore not a scalar boundary measurement.

## Validation

The new example passed scoped release Clippy. Repository formatting, TOML formatting, Rust file-size gates, all 179 Python script tests, crate-dependency guards, shared-field identity checks, external-artifact guards, cargo-machete, spelling, and documentation guardrails passed. The mdbook build was skipped because mdbook is unavailable. The standard generator reproduced the existing fp128 dense recursive catalog byte-for-byte. Every report render checks offload counts against serialized topology and requires one policy digest per field/source/layout series.

## Reproduction

Build `offload_boundary_survey` with `cargo build --release -p akita-planner --features catalog-gen --example offload_boundary_survey`, then run `scripts/survey-offload-boundaries.py --binary <path-to-example>`. Use `--nvs`, `--fields`, `--sources`, and `--layouts` to select cells; layout syntax is chunks:activated-levels. Replay the exact captured cells with `--manifest artifacts/offload-survey/results.tsv` and a fresh `--output` directory. Existing records are skipped unless `--retry` is supplied. Use a fresh output directory when planner source or policy changes. `--timeout` bounds each individual search and records timeouts separately. Run `scripts/report-offload-boundaries.py` to refresh this report.

## Catalog migration

The current single-policy catalogs are separate from this baseline capture. [catalog-migration.json](catalog-migration.json) compares all 27 former offloading rows: 25 schedules are identical; two W2R3 grouped rows have changed producer profiles; all workload shapes remain present and all eight policy digests are unchanged. Reproduce with `scripts/compare-offloading-catalogs.py`. The Book explains the configuration and setup migration.
