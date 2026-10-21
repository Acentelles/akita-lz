# Floor-off default audit

Baseline: `4bdbd42fa806a3286884d4450d2b95b58205505d`. Floor-off policy:
`a125c36c7`; regenerated catalogs: `f2ec5b31c`. All builds, generation,
tests, and benchmarks run through `.remote/run` on the supplied 8-vCPU,
16-GB Boat box. There is no local `target/`.

The opening-basis floor is off by default for every Akita preset. All 27
catalogs and 151 snapshot rows have been regenerated. Total modeled proof
bytes decrease **10,419,942 → 10,335,206 (−0.81%)**. Seven of the 13 profile
cases improve in the model; six retain the same modeled size. Recursive
multi-group FP128 has the largest modeled reduction, **12.34%**.
All thirteen paired profiles pass at their original sizes, with no OOM or
skipped case. Real proof bytes decrease in ten cases and increase in three.
Recursive multi-group FP128 saves **12.75% real bytes**, but some cases are
slower: direct multi-group prover/verifier medians rise **15.03% / 33.74%**,
and W4R2 prover time rises **13.24%**. The required full-regeneration/drift
gate remains blocked by memory use.

The required two-worker full generation was killed near the machine's memory
limit. Complete standard families were recovered in separate processes.
This publishes valid catalogs but does not establish that CI's full
regeneration fits on this box.

## Profile measurements

| Profile case | Real proof bytes: before → after (%) | Prove seconds: before → after (%) | Verify (8 threads), ms: before → after (%) | Baseline outcome | Floor-off outcome |
| --- | ---: | ---: | ---: | --- | --- |
| `dense_fp32:30:1` | 64,548 → 64,537 (-0.0170%) | 2.758 → 2.754 (-0.14%) | 21.971 → 22.105 (+0.61%) | 3 measured runs; peak RSS 4,626,484 KiB | 3 measured runs; peak RSS 4,626,520 KiB |
| `onehot_fp32:34:1` | 65,507 → 65,412 (-0.1450%) | 2.211 → 2.207 (-0.19%) | 26.198 → 25.619 (-2.21%) | 3 measured runs; peak RSS 716,544 KiB | 3 measured runs; peak RSS 742,376 KiB |
| `dense_fp64:29:1` | 66,724 → 66,713 (-0.0165%) | 3.493 → 3.534 (+1.19%) | 14.131 → 12.582 (-10.96%) | 3 measured runs; peak RSS 4,795,220 KiB | 3 measured runs; peak RSS 4,795,188 KiB |
| `onehot_fp64:35:1` | 67,293 → 67,232 (-0.0906%) | 2.058 → 2.120 (+3.04%) | 19.274 → 19.805 (+2.76%) | 3 measured runs; peak RSS 1,368,496 KiB | 3 measured runs; peak RSS 1,368,984 KiB |
| `dense_fp128:28:1` | 69,062 → 69,072 (+0.0145%) | 3.876 → 3.907 (+0.81%) | 16.611 → 16.367 (-1.47%) | 3 measured runs; peak RSS 5,044,752 KiB | 3 measured runs; peak RSS 5,044,112 KiB |
| `onehot_fp128:36:1:direct` | 69,700 → 69,758 (+0.0832%) | 4.197 → 4.153 (-1.07%) | 35.535 → 36.642 (+3.11%) | 3 measured runs; peak RSS 3,145,396 KiB | 3 measured runs; peak RSS 3,127,548 KiB |
| `onehot_fp128:36:1:recursive` | 73,154 → 73,155 (+0.0014%) | 4.859 → 4.715 (-2.95%) | 20.543 → 21.263 (+3.50%) | 3 measured runs; peak RSS 3,594,632 KiB | 3 measured runs; peak RSS 3,594,176 KiB |
| `onehot_fp128_multi_group:34:4:direct` | 69,619 → 69,613 (-0.0086%) | 2.789 → 3.209 (+15.03%) | 49.810 → 66.616 (+33.74%) | 3 measured runs; peak RSS 2,705,044 KiB | 3 measured runs; peak RSS 2,709,364 KiB |
| `onehot_fp128_multi_group_recursive:34:4:recursive` | 86,034 → 75,067 (-12.7473%) | 3.313 → 3.361 (+1.46%) | 19.257 → 21.318 (+10.70%) | 3 measured runs; peak RSS 3,401,896 KiB | 3 measured runs; peak RSS 3,393,884 KiB |
| `onehot_fp128_multi_group_recursive_multi_chunk_w8r2:32:4:recursive` | 75,253 → 74,125 (-1.4989%) | 2.946 → 2.983 (+1.26%) | 27.687 → 25.186 (-9.03%) | 3 measured runs; peak RSS 1,985,508 KiB | 3 measured runs; peak RSS 1,975,232 KiB |
| `onehot_fp128_multi_chunk_w2r2:32:1` | 69,515 → 69,510 (-0.0072%) | 1.136 → 1.094 (-3.68%) | 18.934 → 19.443 (+2.69%) | 3 measured runs; peak RSS 856,508 KiB | 3 measured runs; peak RSS 856,808 KiB |
| `onehot_fp128_multi_chunk_w4r2:32:1` | 70,509 → 70,055 (-0.6439%) | 1.408 → 1.594 (+13.24%) | 22.487 → 24.278 (+7.97%) | 3 measured runs; peak RSS 903,072 KiB | 3 measured runs; peak RSS 862,768 KiB |
| `onehot_fp128_multi_chunk_w8r2:32:1` | 71,887 → 71,873 (-0.0195%) | 2.019 → 1.916 (-5.09%) | 28.912 → 22.805 (-21.12%) | 3 measured runs; peak RSS 988,080 KiB | 3 measured runs; peak RSS 988,816 KiB |

All before/after columns are floor-on baseline → floor-off head. Real bytes
are identical across the three measured samples for each binary/case.
The [benchmark summary](../../artifacts/floor-off-default/benchmark-summary.json)
retains each sample's bytes, timings, RSS, and the diagnostics cited below.

### Procedure

The matrix contains all thirteen cases in `.github/workflows/profile-bench.yml`,
including `onehot_fp128_multi_chunk_w8r2:32:1`. No case is reduced in size.
The unchanged CI runner, `scripts/profile_bench_report.py run`, receives both
binaries, all cases, `--runs 3 --warmups 1`, and separate head/baseline output
directories. Runs are interleaved. Runtime settings are
`AKITA_PROFILE_PROVE_THREADS=8`, `AKITA_PROFILE_VERIFY_THREADS=8`,
`AKITA_PROFILE_TRACE=0`, `AKITA_PROFILE_SPAN_CLOSES=0`,
`AKITA_PROFILE_LOG=info`, `AKITA_PROFILE_ANSI=0`, and
`CARGO_NET_GIT_FETCH_WITH_CLI=true`.

The head build command is
`cargo build --release -p akita-pcs --example profile --no-default-features --features parallel,profile-ci,transcript-blake2b`.
The supplied baseline binary at
`/home/user/akita-baseline/target/release/examples/profile` failed all thirteen
cases during setup with policy mismatch. `strace` confirmed that, even with
`/home/user/akita-baseline` as cwd, it opened
`/home/user/akita-root-basis-5/crates/akita-config/../../artifacts/schedules/fp32_dense.aks`.
Its compiled artifact path therefore points at the head checkout. Merely
changing cwd cannot fix that binary. Those failed pairs are not performance
measurements. The supplementary head-only run completed all thirteen cases.

A fresh baseline build uses the unchanged baseline source with
`--manifest-path /home/user/akita-baseline/Cargo.toml --locked` and a separate
`CARGO_TARGET_DIR=/home/user/akita-root-basis-5/target/baseline-profile`.
The complete paired matrix was rerun successfully with that binary after a
trace confirmed it opens the baseline's own catalog. The rebuild took
2,226.90 seconds and peaked at 10,018,208 KiB RSS. Binary hashes, features,
Rust flags, and the path observation are in the
[benchmark provenance](../../artifacts/floor-off-default/benchmark-provenance.json). The original baseline binary,
lockfile, source, and catalogs are left unchanged. Both builds use the same
features and empty Rust flags. CI additionally sets
`-D warnings -C target-cpu=x86-64-v3`; these measurements compare matching
release builds without those additional flags.

Times are medians of three measured samples after one warmup. Proof bytes come
from the first measured sample, matching the CI aggregator; peak RSS is the
maximum measured sample. Parallel verifier times use eight threads. The CI
formatter also reports single-thread verification. Warmup failures have no
three-sample median. The runner cancels the paired side after a failure;
a copied cancellation is not an independent OOM observation.

### CI-format comparison

Rendered by `scripts/profile_bench_report.py render` with
`--main-baseline-dir` and `--compact`, the CI comment's formatter.

13 of 13 profiles passed. Each sample verifies the same proof with eight
threads and then one thread, reusing verifier setup. The following tables
are copied from the unchanged CI formatter; percentages compare with baseline.

### Phase time

| Profile | Setup | Commit | Prove | Verify, multi-threaded | Verify, single-threaded |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fp32 dense nv30, direct setup check | 0.048 s<br><sub>+7.3%</sub> | 1.882 s<br><sub>-0.4%</sub> | 2.754 s<br><sub>-0.1%</sub> | 22.1 ms<br><sub>+0.6%</sub> | 69.0 ms<br><sub>-1.0%</sub> |
| Fp32 one\-hot nv34, direct setup check | 0.069 s<br><sub>+18.7%</sub> | 1.205 s<br><sub>+0.2%</sub> | 2.207 s<br><sub>-0.2%</sub> | 25.6 ms<br><sub>-2.2%</sub> | 87.9 ms<br><sub>-0.9%</sub> |
| Fp64 dense nv29, direct setup check | 0.117 s<br><sub>+3.4%</sub> | 2.417 s<br><sub>+1.4%</sub> | 3.534 s<br><sub>+1.2%</sub> | 12.6 ms<br><sub>-11.0%</sub> | 39.8 ms<br><sub>-0.0%</sub> |
| Fp64 one\-hot nv35, direct setup check | 0.185 s<br><sub>-6.1%</sub> | 2.212 s<br><sub>+4.3%</sub> | 2.120 s<br><sub>+3.0%</sub> | 19.8 ms<br><sub>+2.8%</sub> | 74.8 ms<br><sub>-4.1%</sub> |
| Fp128 dense nv28, direct setup check | 0.190 s<br><sub>-1.3%</sub> | 3.116 s<br><sub>+20.1%</sub> | 3.907 s<br><sub>+0.8%</sub> | 16.4 ms<br><sub>-1.5%</sub> | 55.3 ms<br><sub>-9.3%</sub> |
| Fp128 one\-hot nv36, direct setup check | 0.582 s<br><sub>+2.3%</sub> | 8.182 s<br><sub>-16.3%</sub> | 4.153 s<br><sub>-1.1%</sub> | 36.6 ms<br><sub>+3.1%</sub> | 90.7 ms<br><sub>-43.0%</sub> |
| Fp128 one\-hot nv36, recursive setup check | 0.937 s<br><sub>-5.3%</sub> | 8.082 s<br><sub>-21.7%</sub> | 4.715 s<br><sub>-3.0%</sub> | 21.3 ms<br><sub>+3.5%</sub> | 65.5 ms<br><sub>+1.9%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), direct setup check | 0.803 s<br><sub>-8.8%</sub> | 5.671 s<br><sub>+4.7%</sub> | 3.209 s<br><sub>+15.0%</sub> | 66.6 ms<br><sub>+33.7%</sub> | 155.0 ms<br><sub>+38.3%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), recursive setup check | 1.568 s<br><sub>+12.7%</sub> | 4.748 s<br><sub>+2.9%</sub> | 3.361 s<br><sub>+1.5%</sub> | 21.3 ms<br><sub>+10.7%</sub> | 35.4 ms<br><sub>-14.4%</sub> |
| Fp128 multi\-group W8R2 \(final nv32, 4 polys total\), recursive setup check | 0.781 s<br><sub>+2.8%</sub> | 1.344 s<br><sub>+2.3%</sub> | 2.983 s<br><sub>+1.3%</sub> | 25.2 ms<br><sub>-9.0%</sub> | 76.1 ms<br><sub>-0.7%</sub> |
| Fp128 one\-hot nv32 W2R2, direct setup check | 0.183 s<br><sub>-11.3%</sub> | 0.491 s<br><sub>-8.4%</sub> | 1.094 s<br><sub>-3.7%</sub> | 19.4 ms<br><sub>+2.7%</sub> | 55.2 ms<br><sub>+0.7%</sub> |
| Fp128 one\-hot nv32 W4R2, direct setup check | 0.179 s<br><sub>-4.9%</sub> | 0.483 s<br><sub>-0.9%</sub> | 1.594 s<br><sub>+13.2%</sub> | 24.3 ms<br><sub>+8.0%</sub> | 61.6 ms<br><sub>+1.6%</sub> |
| Fp128 one\-hot nv32 W8R2, direct setup check | 0.178 s<br><sub>-2.4%</sub> | 0.474 s<br><sub>-13.6%</sub> | 1.916 s<br><sub>-5.1%</sub> | 22.8 ms<br><sub>-21.1%</sub> | 72.8 ms<br><sub>+0.5%</sub> |

### Proof size and protocol shape

| Profile | Fold A/B/D schedule | Total proof | Terminal response | Terminal response planned maximum | Fold levels | Grinding retries |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Fp32 dense nv30, direct setup check | 2048/256/256 → 256/256/256 → 128/128/128 | 64,537 bytes<br><sub>-0.0%</sub> | 38,596 bytes<br><sub>-0.0%</sub> | 40,568 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp32 one\-hot nv34, direct setup check | 2048/256/256 → 512/256/256 → 128/128/128 | 65,412 bytes<br><sub>-0.1%</sub> | 38,598 bytes<br><sub>-0.0%</sub> | 40,568 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp64 dense nv29, direct setup check | 2048/128/128 → 128/128/128 | 66,713 bytes<br><sub>-0.0%</sub> | 40,964 bytes<br><sub>-0.0%</sub> | 42,840 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp64 one\-hot nv35, direct setup check | 512/128/128 → 128/128/128 | 67,232 bytes<br><sub>-0.1%</sub> | 40,981 bytes<br><sub>-0.0%</sub> | 42,840 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp128 dense nv28, direct setup check | 1024/64/64 → 64/64/64 | 69,072 bytes<br><sub>+0.0%</sub> | 48,536 bytes<br><sub>+0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp128 one\-hot nv36, direct setup check | 512/64/64 → 64/64/64 | 69,758 bytes<br><sub>+0.1%</sub> | 48,541 bytes<br><sub>+0.1%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp128 one\-hot nv36, recursive setup check | 512/64/64 → 64/64/64 | 73,155 bytes<br><sub>+0.0%</sub> | 48,535 bytes<br><sub>+0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 8<br><sub>+0.0%</sub> | None |
| Fp128 multi\-group \(final nv34, 4 polys total\), direct setup check | 256/64/64 → 64/64/64 | 69,613 bytes<br><sub>-0.0%</sub> | 48,528 bytes<br><sub>+0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | L3: 0 / 0 / 0<br><sub>Merge base</sub><br>L3: 1 / 1 / 1 |
| Fp128 multi\-group \(final nv34, 4 polys total\), recursive setup check | 512/128/128 → 64/64/64 | 75,067 bytes<br><sub>-12.7%</sub> | 48,480 bytes<br><sub>-7.9%</sub> | 50,919 bytes<br><sub>-7.3%</sub> | 8<br><sub>+0.0%</sub> | None |
| Fp128 multi\-group W8R2 \(final nv32, 4 polys total\), recursive setup check | 256/64/64 → 128/64/64 → 64/64/64 | 74,125 bytes<br><sub>-1.5%</sub> | 48,532 bytes<br><sub>+0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 8<br><sub>+0.0%</sub> | None |
| Fp128 one\-hot nv32 W2R2, direct setup check | 512/64/64 → 128/64/64 → 64/64/64 | 69,510 bytes<br><sub>-0.0%</sub> | 48,541 bytes<br><sub>+0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp128 one\-hot nv32 W4R2, direct setup check | 512/64/64 → 64/64/64 | 70,055 bytes<br><sub>-0.6%</sub> | 48,520 bytes<br><sub>-0.1%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 7<br><sub>+0.0%</sub> | None |
| Fp128 one\-hot nv32 W8R2, direct setup check | 512/64/64 → 128/64/64 → 64/64/64 | 71,873 bytes<br><sub>-0.0%</sub> | 48,512 bytes<br><sub>-0.0%</sub> | 50,919 bytes<br><sub>+0.0%</sub> | 8<br><sub>+0.0%</sub> | None |

Grinding retries are rejected attempts at each fold, listed in measured-run order. Zero means the first sampled nonce was accepted.

Each multi-group profile has two precommitted nv16 singleton polynomials and two polynomials in the displayed final group.

### Native proof accounting

| Profile | Non-nonce proof | Nonce messages | Nonce maximum |
| --- | ---: | ---: | ---: |
| Fp32 dense nv30, direct setup check | 64,068 bytes<br><sub>-0.0%</sub> | 469 bytes<br><sub>+0.2%</sub> | 934 bytes<br><sub>+0.0%</sub> |
| Fp32 one\-hot nv34, direct setup check | 64,918 bytes<br><sub>-0.1%</sub> | 494 bytes<br><sub>+0.4%</sub> | 984 bytes<br><sub>+0.2%</sub> |
| Fp64 dense nv29, direct setup check | 66,244 bytes<br><sub>-0.0%</sub> | 469 bytes<br><sub>+0.2%</sub> | 934 bytes<br><sub>+0.0%</sub> |
| Fp64 one\-hot nv35, direct setup check | 66,757 bytes<br><sub>-0.1%</sub> | 475 bytes<br><sub>+0.2%</sub> | 946 bytes<br><sub>+0.0%</sub> |
| Fp128 dense nv28, direct setup check | 68,728 bytes<br><sub>+0.0%</sub> | 344 bytes<br><sub>-0.3%</sub> | 687 bytes<br><sub>+0.0%</sub> |
| Fp128 one\-hot nv36, direct setup check | 69,405 bytes<br><sub>+0.1%</sub> | 353 bytes<br><sub>+0.0%</sub> | 704 bytes<br><sub>+0.0%</sub> |
| Fp128 one\-hot nv36, recursive setup check | 72,727 bytes<br><sub>+0.0%</sub> | 428 bytes<br><sub>-0.5%</sub> | 854 bytes<br><sub>+0.0%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), direct setup check | 69,264 bytes<br><sub>-0.0%</sub> | 349 bytes<br><sub>-1.1%</sub> | 694 bytes<br><sub>-1.1%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), recursive setup check | 74,608 bytes<br><sub>-12.7%</sub> | 459 bytes<br><sub>-24.5%</sub> | 912 bytes<br><sub>-24.4%</sub> |
| Fp128 multi\-group W8R2 \(final nv32, 4 polys total\), recursive setup check | 73,668 bytes<br><sub>-1.5%</sub> | 457 bytes<br><sub>-8.8%</sub> | 912 bytes<br><sub>-8.4%</sub> |
| Fp128 one\-hot nv32 W2R2, direct setup check | 69,165 bytes<br><sub>+0.0%</sub> | 345 bytes<br><sub>-1.4%</sub> | 688 bytes<br><sub>-1.1%</sub> |
| Fp128 one\-hot nv32 W4R2, direct setup check | 69,704 bytes<br><sub>-0.6%</sub> | 351 bytes<br><sub>-6.9%</sub> | 699 bytes<br><sub>-6.6%</sub> |
| Fp128 one\-hot nv32 W8R2, direct setup check | 71,488 bytes<br><sub>-0.0%</sub> | 385 bytes<br><sub>+0.5%</sub> | 764 bytes<br><sub>+0.0%</sub> |

### Memory and setup size

| Profile | Setup vector | Prepared NTT cache | Verifier NTT cache | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| Fp32 dense nv30, direct setup check | 22.0 MiB<br><sub>+0.0%</sub> | 176.0 MiB<br><sub>+0.0%</sub> | 2.8 MiB<br><sub>+200.0%</sub> | 4518.1 MiB<br><sub>+0.0%</sub> |
| Fp32 one\-hot nv34, direct setup check | 32.0 MiB<br><sub>+0.0%</sub> | 248.0 MiB<br><sub>+10.7%</sub> | 3.0 MiB<br><sub>+0.0%</sub> | 725.0 MiB<br><sub>+3.6%</sub> |
| Fp64 dense nv29, direct setup check | 32.0 MiB<br><sub>+0.0%</sub> | 306.0 MiB<br><sub>+0.0%</sub> | 0.8 MiB<br><sub>+0.0%</sub> | 4682.8 MiB<br><sub>-0.0%</sub> |
| Fp64 one\-hot nv35, direct setup check | 128.0 MiB<br><sub>+0.0%</sub> | 648.0 MiB<br><sub>+0.0%</sub> | 0.8 MiB<br><sub>+0.0%</sub> | 1336.9 MiB<br><sub>+0.0%</sub> |
| Fp128 dense nv28, direct setup check | 64.0 MiB<br><sub>+0.0%</sub> | 513.0 MiB<br><sub>+0.0%</sub> | 1.2 MiB<br><sub>+0.0%</sub> | 4925.9 MiB<br><sub>-0.0%</sub> |
| Fp128 one\-hot nv36, direct setup check | 256.0 MiB<br><sub>+0.0%</sub> | 1796.0 MiB<br><sub>+0.0%</sub> | 3.2 MiB<br><sub>+0.0%</sub> | 3054.2 MiB<br><sub>-0.6%</sub> |
| Fp128 one\-hot nv36, recursive setup check | 256.0 MiB<br><sub>+0.0%</sub> | 1836.0 MiB<br><sub>+0.0%</sub> | 1.2 MiB<br><sub>+0.0%</sub> | 3509.9 MiB<br><sub>-0.0%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), direct setup check | 256.0 MiB<br><sub>+0.0%</sub> | 1797.0 MiB<br><sub>+0.0%</sub> | 3.2 MiB<br><sub>+0.0%</sub> | 2645.9 MiB<br><sub>+0.2%</sub> |
| Fp128 multi\-group \(final nv34, 4 polys total\), recursive setup check | 256.0 MiB<br><sub>+0.0%</sub> | 1887.0 MiB<br><sub>-0.3%</sub> | 1.2 MiB<br><sub>-25.0%</sub> | 3314.3 MiB<br><sub>-0.2%</sub> |
| Fp128 multi\-group W8R2 \(final nv32, 4 polys total\), recursive setup check | 128.0 MiB<br><sub>+0.0%</sub> | 986.2 MiB<br><sub>+0.0%</sub> | 1.2 MiB<br><sub>+0.0%</sub> | 1928.9 MiB<br><sub>-0.5%</sub> |
| Fp128 one\-hot nv32 W2R2, direct setup check | 64.0 MiB<br><sub>+0.0%</sub> | 485.0 MiB<br><sub>+0.0%</sub> | 1.2 MiB<br><sub>-25.0%</sub> | 836.7 MiB<br><sub>+0.0%</sub> |
| Fp128 one\-hot nv32 W4R2, direct setup check | 64.0 MiB<br><sub>+0.0%</sub> | 449.0 MiB<br><sub>-7.4%</sub> | 1.2 MiB<br><sub>+0.0%</sub> | 842.5 MiB<br><sub>-4.5%</sub> |
| Fp128 one\-hot nv32 W8R2, direct setup check | 64.0 MiB<br><sub>+0.0%</sub> | 485.0 MiB<br><sub>+0.0%</sub> | 1.2 MiB<br><sub>-25.0%</sub> | 965.6 MiB<br><sub>+0.1%</sub> |

Deltas are shown only for profiles with a matching merge-base case. Negative is smaller or faster.

### Regressions and interpretation

Every positive headline delta is listed here; a smaller modeled proof does
not imply faster proving or verification.

- `dense_fp32:30:1`: eight-thread verifier time +0.61%.
- `dense_fp64:29:1`: prover time +1.19%.
- `onehot_fp64:35:1`: prover time +3.04%; eight-thread verifier time +2.76%.
- `dense_fp128:28:1`: real proof bytes +0.01%; prover time +0.81%.
- `onehot_fp128:36:1:direct`: real proof bytes +0.08%; eight-thread verifier time +3.11%.
- `onehot_fp128:36:1:recursive`: real proof bytes +1 byte (+0.0014%); eight-thread verifier time +3.50%.
- `onehot_fp128_multi_group:34:4:direct`: prover time +15.03%; eight-thread verifier time +33.74%.
- `onehot_fp128_multi_group_recursive:34:4:recursive`: prover time +1.46%; eight-thread verifier time +10.70%.
- `onehot_fp128_multi_group_recursive_multi_chunk_w8r2:32:4:recursive`: prover time +1.26%.
- `onehot_fp128_multi_chunk_w2r2:32:1`: eight-thread verifier time +2.69%.
- `onehot_fp128_multi_chunk_w4r2:32:1`: prover time +13.24%; eight-thread verifier time +7.97%.

The three real-byte increases are fully accounted for by observed terminal
response and nonce lengths, while their modeled terminal response cap stays
50,919 bytes:

| Case | Actual terminal response bytes, before → after | Nonce bytes, before → after | Net proof increase |
| --- | ---: | ---: | ---: |
| FP128 dense nv28 | 48,525 → 48,536 | 345 → 344 | +10 bytes |
| FP128 one-hot nv36, direct | 48,483 → 48,541 | 353 → 353 | +58 bytes |
| FP128 one-hot nv36, recursive | 48,532 → 48,535 | 430 → 428 | +1 byte |

The model prices a bound, whereas the actual terminal-response encoding
length depends on the resulting response. These small increases are real,
repeat across all three samples, and must not be rounded into an unchanged
proof claim.

The major timing tradeoffs deserve separate attention:

- Direct multi-group saves only six real bytes. Its L3/L4 A ranks increase
  from 3 to 4 and their security route changes from L2 to L-infinity. All
  three floor-off prover samples (3.068–3.279 s) exceed all three baseline
  samples (2.563–2.997 s). The verifier also shows substantial variability:
  even the unchanged quotient-prefix setup scan rises from a median
  20.710 to 36.421 ms. Thus the measured regression is clear, but these
  samples do not isolate how much is caused by the changed suffix versus
  execution variability. No timing improvement is claimed for this case.
- Recursive multi-group saves 10,967 bytes but is 1.46% slower to prove and
  10.70% slower to verify with eight threads. Dropping the opening log basis
  from 6 to 3 at L2 grows the next witness from 1,400,832 to 2,332,928 fields.
  The measured reduced-relation setup scan rises 3.799 → 5.057 ms. The
  schedule trades serialized size for different intermediate work; the
  objective does not require that work to become faster.
- W4R2 saves 454 bytes but its prover median rises 13.24%. L1's A dimension
  changes 128 → 64 with rank 3 → 5, and the L2 next witness grows
  1,310,720 → 1,680,000 fields. These are material arithmetic-shape changes
  behind a 0.64% byte reduction. Its three prover samples increase from
  1.406–1.429 s to 1.478–1.658 s. The precise contribution of each changed
  level is not isolated by this benchmark.

The CI table also reports single-thread verifier regressions: `onehot_fp128:36:1:recursive` (+1.87%), `onehot_fp128_multi_group:34:4:direct` (+38.32%), `onehot_fp128_multi_chunk_w2r2:32:1` (+0.75%), `onehot_fp128_multi_chunk_w4r2:32:1` (+1.59%), `onehot_fp128_multi_chunk_w8r2:32:1` (+0.49%). Their sample ranges overlap; these medians are retained without selecting replacement runs.

The smaller positive headline timing deltas (up to 3.50%) have overlapping three-sample
ranges. They remain regressions in the reported medians; the sample count
does not establish their statistical significance or a unique cause. The
planner optimizes modeled payload after its setup priorities, not elapsed
prover/verifier time. No extra runs were selected to replace these results.

Modeled cost and serialized bytes are separate measurements. Equal modeled
bytes do not imply identical schedules: terminal fold bases can change even
where all nonterminal bases stay fixed. Three repetitions describe this run,
not a statistical performance guarantee.

## Regeneration cost

The owner updated full generation to `AKITA_SCHEDULE_GEN_JOBS=2`, matching
CI and `.remote/drift.sh`. Compilation uses `CARGO_BUILD_JOBS=1`.
Measurements use remote `/usr/bin/time -v`.

| Policy / run | Outcome | Wall seconds | Peak RSS, KiB |
| --- | --- | ---: | ---: |
| Floor on, two workers | Completed | 1,755.00 | 10,912,608 |
| Floor off, two workers | SIGKILL; wrapper exit 137 | 3,619 | 15,666,868 |
| Floor on, four workers | Owner-reported successful run | about 890–1,000 | not supplied |
| Floor off, four workers, row diagnostics | SIGKILL; wrapper exit 137 | 1,702.57 | 15,636,136 |
| Floor off, four workers, allocator tuning | SIGKILL; wrapper exit 137 | 2,540.86 | 15,668,604 |
| Floor off, four workers, ordinary generator | SIGKILL; wrapper exit 137 | 1,423.03 | 15,666,788 |

There is **no successful two-worker floor-off full-regeneration time**.
It ran for more than twice the successful floor-on time before being killed.
Repeated signal 9 near physical-memory capacity is consistent with OOM;
kernel logs were inaccessible, so the kernel's kill reason was not confirmed.
GNU time prints `Exit status: 0` after its signal diagnostic; the actual wrapper
exit was 137. The first four-worker measurement includes 17.34 seconds of
compilation. The allocator retry used `MALLOC_ARENA_MAX=2`,
`MALLOC_TRIM_THRESHOLD_=131072`, and `MALLOC_MMAP_THRESHOLD_=131072` for that
invocation only.

The baseline measurement uses its existing generator without modifying its
checkout. All 27 output catalogs are byte-identical to `4bdbd42fa`; all 151
keyed TSV rows match. Raw TSV ordering differs because the old SmallDense
additions were appended. The raw comparison's exit 1 is not a generation
failure. Internal baseline generator time was 1,754.82 seconds.

Fallback recovery used the unchanged generator separately for each complete
standard family, with `--catalog-snapshot` and no explicit row overrides.
All 27 outputs were published; their snapshots were joined by family/key.
Successful per-family command times sum to **5,378.12 seconds**; the largest
process peak RSS is **7,719,596 KiB**. Worker counts were four for the 21 FP128
families, two for FP32 dense/one-hot and FP64 dense/one-hot, and one for
FP32/FP64 recursive dense. These mixed-worker, fresh-process timings are not
a full two-worker measurement. An earlier `--final-group` combined with
`--catalog-snapshot` invocation was rejected for incompatible flags before
planning; it is not a planner failure.

Exact measurements are retained in the small
[regeneration summary](../../artifacts/floor-off-default/regeneration-summary.json).

The required `bash .remote/drift.sh` was also run with its two-worker default.
It was killed with exit **137** after **4,196 seconds (1:09:56)**, at
**15,666,040 KiB** peak RSS. It did not reach `DRIFT_OK`. Consequently the
full-regeneration acceptance gate remains blocked, despite successful catalog
publication and runtime/security validation. No storage optimization has been
applied: the original source scope allows policy/default/digest changes only.

## Policy, digest, and fixtures

`CommitmentConfig::monotone_opening_basis()` defaults to `false`, as do manually
constructed default `PlannerPolicy` values. DenseSmall widths 3/4 follow that
default; widths 5/6 retain root opening log basis 5 and remain floor-off.
Presets can still opt into monotonicity.

The policy preimage domain changes from `AKITA-PLANNER-POLICY-V1` to
`AKITA-PLANNER-POLICY-V2`. Root override `None` with floor off requires no
extension. A root override or floor-on opt-in adds `AKITA-OPENING-POLICY-V2`,
followed by the optional root and monotonicity boolean. The outer tag prevents
the new default from aliasing old floor-on identities. Artifact envelope
version and protocol epoch are unchanged. Identity tests cover the default,
both options, and restoration of the default.

Both setup-prefix fixtures were re-derived and remain **unchanged**: count 3
with digest `643eb959607f2fa91a51cb4e445ac706c291481c02edc47d3e6836d6185ea58d`,
and count 3 with digest
`0ea09bafa95cc857b27075caefeaa5dda6c8fb07ade9216198aac783c6384a59`.
Exact assertions are retained. A malformed-digit test now targets the last
(final) root group because a grouped row sorts first in the new catalog;
its rejection assertions are unchanged. No protocol, prover, verifier, or
params implementation changed.

## Profile schedules and modeled bytes

`final=nv:n` identifies the final group's variable count and polynomial count;
`precommitted=` lists preceding producer groups. Multi-group profiles have
a two-polynomial final group and two precommitted singleton groups.
`Lk[(inner,open),...]` lists base-2 logarithms of bases for all groups at
nonterminal level k, in catalog order: prefix/precommitted groups before the
final group. `T(inner,fold)` records terminal bases separately; a terminal has
no opening basis.

| Profile case | Catalog family / logical key | Modeled bytes: before → after (%) | Bases before | Bases after |
| --- | --- | ---: | --- | --- |
| `dense_fp32:30:1` | `fp32_dense` / `final=30:1;precommitted=` | 66,974 → 66,974 (+0.00%) | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| `onehot_fp32:34:1` | `fp32_onehot` / `final=34:1;precommitted=` | 67,950 → 67,872 (-0.11%) | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,6)] L4[(6,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| `dense_fp64:29:1` | `fp64_dense` / `final=29:1;precommitted=` | 69,054 → 69,054 (+0.00%) | `L0[(16,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(16,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| `onehot_fp64:35:1` | `fp64_onehot` / `final=35:1;precommitted=` | 69,610 → 69,562 (-0.07%) | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| `dense_fp128:28:1` | `fp128_dense` / `final=28:1;precommitted=` | 71,798 → 71,798 (+0.00%) | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| `onehot_fp128:36:1:direct` | `fp128_onehot` / `final=36:1;precommitted=` | 72,487 → 72,487 (+0.00%) | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| `onehot_fp128:36:1:recursive` | `fp128_onehot_recursive` / `final=36:1;precommitted=` | 75,965 → 75,965 (+0.00%) | `L0[(3,3)] L1[(13,3),(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(13,3),(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| `onehot_fp128_multi_group:34:4:direct` | `fp128_onehot` / `final=34:2;precommitted=16:1,16:1` | 72,373 → 72,349 (-0.03%) | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| `onehot_fp128_multi_group_recursive:34:4:recursive` | `fp128_onehot_recursive` / `final=34:2;precommitted=16:1,16:1` | 88,933 → 77,959 (-12.34%) | `L0[(3,3),(3,3),(3,3)] L1[(10,6),(3,6)] L2[(6,6)] L3[(6,6)] L4[(6,6)] L5[(6,6)] L6[(6,6)] T(6,6)` | `L0[(3,3),(3,3),(3,3)] L1[(10,6),(3,6)] L2[(6,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,3)` |
| `onehot_fp128_multi_group_recursive_multi_chunk_w8r2:32:4:recursive` | `fp128_onehot_recursive_multi_chunk_w8r2` / `final=32:2;precommitted=16:1,16:1` | 78,139 → 76,967 (-1.50%) | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| `onehot_fp128_multi_chunk_w2r2:32:1` | `fp128_onehot_multi_chunk_w2r2` / `final=32:1;precommitted=` | 72,255 → 72,231 (-0.03%) | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| `onehot_fp128_multi_chunk_w4r2:32:1` | `fp128_onehot_multi_chunk_w4r2` / `final=32:1;precommitted=` | 73,235 → 72,802 (-0.59%) | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| `onehot_fp128_multi_chunk_w8r2:32:1` | `fp128_onehot_multi_chunk` / `final=32:1;precommitted=` | 74,659 → 74,659 (+0.00%) | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,3)` |

## Complete catalog comparison

All 151 rows are matched by family and logical key and sorted in that order.
The bounded-producer suffix distinguishes equal group sizes with different
producer contracts. Totals sum catalog rows; they are not a single proof or
a benchmark-weighted aggregate.

One modeled proof cost increases: `fp128_dense_recursive`,
`final=22:1;precommitted=`, **69,197 → 74,163 (+7.18%)**. The existing recursive
selection policy first minimizes the padded total setup envelope, then
first-direct setup capacity, then proof/work cost. Here the padded envelope
stays 524,288 fields while first-direct capacity falls **524,288 → 262,144**.
That higher-priority improvement wins despite raw setup fields increasing
393,216 → 524,288 and fold levels increasing six → seven. The ordering is
`CompleteObjectiveBound` in
`crates/akita-planner/src/schedule_params/objective.rs`; it was not changed.
Proof bytes are the payload objective, but not the first coordinate of every
existing selection objective. Across all rows, 56 modeled proof costs decrease,
one increases, and 94 are unchanged.

| Family | Logical key | Modeled proof bytes: before → after (%) | Setup fields: before → after (%) | Fold levels: before → after | Bases before | Bases after |
| --- | --- | ---: | ---: | ---: | --- | --- |
| fp128_dense | `final=14:1;precommitted=` | 65,074 → 65,074 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(13,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(13,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense | `final=15:2;precommitted=` | 67,327 → 65,832 (-2.22%) | 165,120 → 131,072 (-20.62%) | 6 → 5 | `L0[(13,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(13,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_dense | `final=16:1;precommitted=` | 67,325 → 65,830 (-2.22%) | 165,120 → 131,072 (-20.62%) | 6 → 5 | `L0[(13,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(13,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_dense | `final=16:1;precommitted=14:1` | 67,929 → 67,752 (-0.26%) | 163,840 → 131,072 (-20.00%) | 6 → 5 | `L0[(13,3),(16,3)] L1[(3,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(13,3),(16,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_dense | `final=16:2;precommitted=` | 67,573 → 65,949 (-2.40%) | 209,152 → 131,072 (-37.33%) | 6 → 5 | `L0[(13,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(13,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_dense | `final=17:4;precommitted=` | 68,369 → 68,369 (+0.00%) | 204,800 → 204,800 (+0.00%) | 6 → 6 | `L0[(13,3)] L1[(3,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(13,3)] L1[(3,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense | `final=24:1;precommitted=` | 70,995 → 69,985 (-1.42%) | 786,432 → 786,432 (+0.00%) | 7 → 6 | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,5)` | `L0[(11,3)] L1[(3,5)] L2[(5,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense | `final=26:1;precommitted=` | 71,567 → 71,567 (+0.00%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_dense | `final=28:1;precommitted=` | 71,798 → 71,798 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_dense | `final=30:1;precommitted=` | 72,371 → 72,346 (-0.03%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_dense | `final=32:1;precommitted=` | 72,486 → 72,486 (+0.00%) | 16,777,216 → 16,777,216 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_bounded | `final=14:1;precommitted=` | 64,380 → 64,380 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(14,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,5)` | `L0[(14,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_bounded | `final=24:1;precommitted=` | 70,995 → 69,985 (-1.42%) | 786,432 → 786,432 (+0.00%) | 7 → 6 | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,5)` | `L0[(11,3)] L1[(3,5)] L2[(5,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_bounded | `final=26:1;precommitted=` | 71,226 → 71,226 (+0.00%) | 1,572,864 → 1,572,864 (+0.00%) | 7 → 7 | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,5)` | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_multi_chunk | `final=16:1;precommitted=` | 71,218 → 71,218 (+0.00%) | 655,360 → 655,360 (+0.00%) | 7 → 7 | `L0[(10,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(10,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_multi_chunk_w8r3 | `final=16:1;precommitted=` | 74,071 → 74,071 (+0.00%) | 1,169,600 → 1,169,600 (+0.00%) | 8 → 8 | `L0[(10,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(10,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_dense_recursive | `final=20:1;precommitted=` | 69,081 → 68,683 (-0.58%) | 188,416 → 262,144 (+39.13%) | 6 → 6 | `L0[(12,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(12,3)] L1[(3,3)] L2[(3,5)] L3[(5,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_recursive | `final=22:1;precommitted=` | 69,197 → 74,163 (+7.18%) | 393,216 → 524,288 (+33.33%) | 6 → 7 | `L0[(11,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(11,3)] L1[(11,6),(3,6)] L2[(6,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_recursive | `final=24:1;precommitted=` | 70,995 → 69,985 (-1.42%) | 786,432 → 786,432 (+0.00%) | 7 → 6 | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,5)` | `L0[(11,3)] L1[(3,5)] L2[(5,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_recursive | `final=26:1;precommitted=` | 73,961 → 73,679 (-0.38%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(12,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(12,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_recursive | `final=28:1;precommitted=` | 74,751 → 74,319 (-0.58%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(13,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(16,3)] L1[(13,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_small_3 | `final=14:1;precommitted=` | 49,255 → 49,255 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp128_dense_small_4 | `final=14:1;precommitted=` | 49,255 → 49,255 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(4,3)] T(3,3)` | `L0[(4,3)] T(3,3)` |
| fp128_dense_small_5 | `final=13:1;precommitted=` | 51,467 → 51,467 (+0.00%) | 65,536 → 65,536 (+0.00%) | 2 → 2 | `L0[(5,5)] T(5,3)` | `L0[(5,5)] T(5,3)` |
| fp128_dense_small_5 | `final=14:1;precommitted=` | 61,707 → 61,707 (+0.00%) | 65,536 → 65,536 (+0.00%) | 2 → 2 | `L0[(5,5)] T(5,3)` | `L0[(5,5)] T(5,3)` |
| fp128_dense_small_5 | `final=15:1;precommitted=` | 65,818 → 65,818 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=16:1;precommitted=` | 65,819 → 65,819 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=17:1;precommitted=` | 66,000 → 66,000 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=18:1;precommitted=` | 65,700 → 65,700 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=19:1;precommitted=` | 66,394 → 66,394 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=20:1;precommitted=` | 66,394 → 66,394 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=21:1;precommitted=` | 67,961 → 67,961 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=22:1;precommitted=` | 67,961 → 67,961 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(5,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=23:1;precommitted=` | 69,572 → 69,572 (+0.00%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,4)] L2[(4,3)] L3[(3,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,3)] L3[(3,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=24:1;precommitted=` | 69,703 → 69,703 (+0.00%) | 262,144 → 262,144 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=25:1;precommitted=` | 69,885 → 69,885 (+0.00%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=26:1;precommitted=` | 69,885 → 69,885 (+0.00%) | 524,288 → 524,288 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=27:1;precommitted=` | 70,499 → 70,499 (+0.00%) | 524,288 → 524,288 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,3)] L2[(3,5)] L3[(5,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,5)] L3[(5,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=28:1;precommitted=` | 70,931 → 70,931 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 6 → 6 | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_5 | `final=29:1;precommitted=` | 72,761 → 72,761 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 7 → 7 | `L0[(5,5)] L1[(5,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,3)` | `L0[(5,5)] L1[(5,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=13:1;precommitted=` | 56,587 → 56,587 (+0.00%) | 65,536 → 65,536 (+0.00%) | 2 → 2 | `L0[(6,5)] T(5,3)` | `L0[(6,5)] T(5,3)` |
| fp128_dense_small_6 | `final=14:1;precommitted=` | 61,707 → 61,707 (+0.00%) | 65,536 → 65,536 (+0.00%) | 2 → 2 | `L0[(6,5)] T(5,3)` | `L0[(6,5)] T(5,3)` |
| fp128_dense_small_6 | `final=15:1;precommitted=` | 65,818 → 65,818 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=16:1;precommitted=` | 65,819 → 65,819 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=17:1;precommitted=` | 66,000 → 66,000 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=18:1;precommitted=` | 65,700 → 65,700 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=19:1;precommitted=` | 66,394 → 66,394 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=20:1;precommitted=` | 66,394 → 66,394 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=21:1;precommitted=` | 67,961 → 67,961 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=22:1;precommitted=` | 67,961 → 67,961 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(6,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,6)] L2[(6,4)] L3[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=23:1;precommitted=` | 69,572 → 69,572 (+0.00%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,4)] L2[(4,3)] L3[(3,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,3)] L3[(3,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=24:1;precommitted=` | 69,703 → 69,703 (+0.00%) | 262,144 → 262,144 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=25:1;precommitted=` | 69,885 → 69,885 (+0.00%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=26:1;precommitted=` | 70,001 → 70,001 (+0.00%) | 524,288 → 524,288 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=27:1;precommitted=` | 70,781 → 70,781 (+0.00%) | 524,288 → 524,288 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=28:1;precommitted=` | 72,000 → 72,000 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 6 → 6 | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_dense_small_6 | `final=29:1;precommitted=` | 72,761 → 72,761 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 7 → 7 | `L0[(6,5)] L1[(5,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,3)` | `L0[(6,5)] L1[(5,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=12:1;precommitted=` | 41,062 → 41,062 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp128_onehot | `final=14:1;precommitted=` | 41,063 → 41,063 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp128_onehot | `final=14:2;precommitted=` | 49,256 → 49,256 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp128_onehot | `final=15:1;precommitted=` | 41,063 → 41,063 (+0.00%) | 49,152 → 49,152 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp128_onehot | `final=15:4;precommitted=` | 64,382 → 64,382 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=16:1;precommitted=` | 64,731 → 64,731 (+0.00%) | 131,072 → 131,072 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,5)` | `L0[(3,3)] T(3,5)` |
| fp128_onehot | `final=16:1;precommitted=14:1` | 64,382 → 64,382 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3),(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,5)` | `L0[(3,3),(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=16:1;precommitted=14:1,15:2` | 67,327 → 65,833 (-2.22%) | 176,128 → 131,072 (-25.58%) | 6 → 5 | `L0[(3,3),(13,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(3,3),(13,3),(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=16:1;precommitted=14:1;producer_contracts=balanced(bound=65)` | 65,076 → 65,076 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(14,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(14,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=16:2;precommitted=` | 64,552 → 64,552 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=18:1;precommitted=` | 64,381 → 64,381 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:1;precommitted=` | 65,074 → 65,074 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:1;precommitted=14:1` | 65,076 → 65,076 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:1;precommitted=14:2` | 65,076 → 65,076 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:2;precommitted=` | 65,192 → 65,192 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:2;precommitted=14:1` | 66,995 → 65,636 (-2.03%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] T(5,5)` | `L0[(3,3),(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:4;precommitted=` | 67,327 → 65,833 (-2.22%) | 165,120 → 131,072 (-20.62%) | 6 → 5 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:4;precommitted=14:1,14:1` | 67,327 → 65,833 (-2.22%) | 176,128 → 131,072 (-25.58%) | 6 → 5 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=20:4;precommitted=14:1,14:1,14:1` | 67,573 → 65,949 (-2.40%) | 209,152 → 131,072 (-37.33%) | 6 → 5 | `L0[(3,3),(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3),(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot | `final=28:1;precommitted=` | 69,347 → 69,347 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_onehot | `final=30:1;precommitted=` | 70,994 → 70,167 (-1.16%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 6 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_onehot | `final=30:4;precommitted=` | 71,569 → 71,569 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=32:1;precommitted=` | 71,567 → 71,567 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=32:2;precommitted=16:1,16:1` | 71,684 → 71,684 (+0.00%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=32:4;precommitted=` | 72,373 → 72,349 (-0.03%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=34:2;precommitted=16:1,16:1` | 72,373 → 72,349 (-0.03%) | 16,777,216 → 16,777,216 (+0.00%) | 7 → 7 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=36:1;precommitted=` | 72,487 → 72,487 (+0.00%) | 16,777,216 → 16,777,216 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=40:1;precommitted=` | 74,432 → 74,287 (-0.19%) | 90,177,536 → 90,177,536 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot | `final=44:1;precommitted=` | 75,579 → 75,579 (+0.00%) | 360,710,144 → 360,710,144 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot | `final=50:1;precommitted=` | 76,731 → 76,707 (-0.03%) | 2,885,681,152 → 2,885,681,152 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,5)] L5[(5,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk | `final=16:1;precommitted=` | 78,376 → 71,621 (-8.62%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,4)] L4[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk | `final=32:1;precommitted=` | 74,659 → 74,659 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk | `final=32:2;precommitted=16:1,16:1` | 75,116 → 75,116 (+0.00%) | 5,812,224 → 5,812,224 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk | `final=34:2;precommitted=16:1,16:1` | 75,233 → 75,233 (+0.00%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r2 | `final=14:1;precommitted=` | 72,623 → 65,598 (-9.67%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r2 | `final=14:1;precommitted=14:1` | 68,097 → 65,716 (-3.50%) | 131,072 → 131,072 (+0.00%) | 6 → 5 | `L0[(3,3),(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] T(5,5)` | `L0[(3,3),(3,3)] L1[(3,5)] L2[(5,4)] L3[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r2 | `final=32:1;precommitted=` | 72,255 → 72,231 (-0.03%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r3 | `final=14:1;precommitted=` | 77,199 → 70,167 (-9.11%) | 163,840 → 163,840 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,4)] L4[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r3 | `final=16:1;precommitted=` | 78,764 → 73,286 (-6.95%) | 163,840 → 163,840 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,4)] L4[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w2r3 | `final=32:1;precommitted=` | 73,668 → 73,185 (-0.66%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,4)] L4[(4,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w4r2 | `final=32:1;precommitted=` | 73,235 → 72,802 (-0.59%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w4r3 | `final=16:1;precommitted=` | 80,660 → 73,748 (-8.57%) | 327,680 → 464,640 (+41.80%) | 7 → 7 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,3)] L4[(3,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w4r3 | `final=32:1;precommitted=` | 74,883 → 74,883 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w8r3 | `final=16:1;precommitted=` | 83,318 → 76,686 (-7.96%) | 858,880 → 858,880 (+0.00%) | 7 → 8 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w8r3 | `final=32:1;precommitted=` | 75,571 → 75,571 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w8r3 | `final=32:2;precommitted=16:1,16:1` | 75,688 → 75,688 (+0.00%) | 5,812,224 → 5,812,224 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_multi_chunk_w8r3 | `final=34:2;precommitted=16:1,16:1` | 75,921 → 75,921 (+0.00%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive | `final=32:2;precommitted=16:1,16:1` | 74,787 → 74,355 (-0.58%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,4)] L5[(4,5)] T(5,3)` |
| fp128_onehot_recursive | `final=34:2;precommitted=16:1,16:1` | 88,933 → 77,959 (-12.34%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(10,6),(3,6)] L2[(6,6)] L3[(6,6)] L4[(6,6)] L5[(6,6)] L6[(6,6)] T(6,6)` | `L0[(3,3),(3,3),(3,3)] L1[(10,6),(3,6)] L2[(6,3)] L3[(3,3)] L4[(3,3)] L5[(3,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive | `final=36:1;precommitted=` | 75,965 → 75,965 (+0.00%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3)] L1[(13,3),(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3)] L1[(13,3),(3,3)] L2[(3,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w2r3 | `final=32:2;precommitted=16:1,16:1` | 78,197 → 76,701 (-1.91%) | 8,388,608 → 8,388,608 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(13,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(13,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w2r3 | `final=34:2;precommitted=16:1,16:1` | 78,347 → 76,851 (-1.91%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w4r3 | `final=32:2;precommitted=16:1,16:1` | 78,196 → 77,248 (-1.21%) | 8,388,608 → 8,388,608 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,5)] L5[(5,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w4r3 | `final=34:2;precommitted=16:1,16:1` | 78,647 → 77,573 (-1.37%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(13,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(13,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w8r2 | `final=32:2;precommitted=16:1,16:1` | 78,139 → 76,967 (-1.50%) | 8,388,608 → 8,388,608 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w8r2 | `final=34:2;precommitted=16:1,16:1` | 80,218 → 79,470 (-0.93%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(16,4),(3,4)] L2[(7,4),(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(16,4),(3,4)] L2[(7,4),(4,4)] L3[(4,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w8r3 | `final=32:2;precommitted=16:1,16:1` | 78,613 → 77,877 (-0.94%) | 8,388,608 → 8,388,608 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp128_onehot_recursive_multi_chunk_w8r3 | `final=34:2;precommitted=16:1,16:1` | 79,403 → 78,655 (-0.94%) | 16,777,216 → 16,777,216 (+0.00%) | 8 → 8 | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,4)] L5[(4,4)] L6[(4,5)] T(5,5)` | `L0[(3,3),(3,3),(3,3)] L1[(12,4),(3,4)] L2[(4,4)] L3[(4,3)] L4[(3,4)] L5[(4,4)] L6[(4,5)] T(5,3)` |
| fp32_dense | `final=20:1;precommitted=` | 62,779 → 61,935 (-1.34%) | 458,752 → 292,864 (-36.16%) | 6 → 6 | `L0[(8,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,5)] L2[(5,3)] L3[(3,3)] L4[(3,5)] T(5,3)` |
| fp32_dense | `final=20:1;precommitted=20:1` | 63,607 → 63,607 (+0.00%) | 524,288 → 524,288 (+0.00%) | 6 → 6 | `L0[(8,3),(8,3)] L1[(3,3)] L2[(3,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(8,3),(8,3)] L1[(3,3)] L2[(3,6)] L3[(6,6)] L4[(6,6)] T(6,3)` |
| fp32_dense | `final=26:1;precommitted=` | 65,383 → 65,326 (-0.09%) | 1,310,720 → 1,048,576 (-20.00%) | 6 → 7 | `L0[(8,3)] L1[(3,4)] L2[(4,4)] L3[(4,5)] L4[(5,6)] T(6,6)` | `L0[(8,3)] L1[(3,4)] L2[(4,3)] L3[(3,3)] L4[(3,3)] L5[(3,5)] T(5,3)` |
| fp32_dense | `final=28:1;precommitted=` | 66,858 → 66,014 (-1.26%) | 2,883,584 → 2,883,584 (+0.00%) | 7 → 7 | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,3)] L2[(3,5)] L3[(5,3)] L4[(3,3)] L5[(3,5)] T(5,3)` |
| fp32_dense | `final=30:1;precommitted=` | 66,974 → 66,974 (+0.00%) | 5,767,168 → 5,767,168 (+0.00%) | 7 → 7 | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp32_dense_recursive | `final=20:1;precommitted=` | 160,700 → 160,700 (+0.00%) | 131,072 → 131,072 (+0.00%) | 3 → 3 | `L0[(8,3)] L1[(3,5)] T(5,6)` | `L0[(8,3)] L1[(3,5)] T(5,3)` |
| fp32_dense_recursive | `final=22:1;precommitted=` | 64,991 → 64,991 (+0.00%) | 262,144 → 262,144 (+0.00%) | 6 → 6 | `L0[(8,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,3)` |
| fp32_dense_recursive | `final=24:1;precommitted=` | 68,535 → 67,124 (-2.06%) | 524,288 → 524,288 (+0.00%) | 6 → 7 | `L0[(8,3)] L1[(8,5),(3,5)] L2[(5,6)] L3[(6,6)] L4[(6,6)] T(6,6)` | `L0[(8,3)] L1[(8,5),(3,5)] L2[(5,4)] L3[(4,3)] L4[(3,3)] L5[(3,5)] T(5,3)` |
| fp32_dense_recursive | `final=26:1;precommitted=` | 68,736 → 68,238 (-0.72%) | 1,048,576 → 1,048,576 (+0.00%) | 7 → 7 | `L0[(8,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] L5[(5,6)] T(6,6)` | `L0[(8,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp32_dense_recursive | `final=28:1;precommitted=` | 69,320 → 68,606 (-1.03%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(8,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(8,3)] L1[(8,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp32_dense_recursive | `final=30:1;precommitted=` | 69,772 → 69,772 (+0.00%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(7,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,6)] L5[(6,6)] T(6,6)` | `L0[(7,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,6)] L5[(6,6)] T(6,3)` |
| fp32_onehot | `final=14:1;precommitted=` | 39,095 → 39,095 (+0.00%) | 98,304 → 98,304 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,3)` | `L0[(3,3)] T(3,3)` |
| fp32_onehot | `final=16:1;precommitted=` | 55,629 → 55,629 (+0.00%) | 212,992 → 212,992 (+0.00%) | 2 → 2 | `L0[(3,3)] T(3,5)` | `L0[(3,3)] T(3,5)` |
| fp32_onehot | `final=16:2;precommitted=` | 56,284 → 56,284 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,3)` |
| fp32_onehot | `final=20:1;precommitted=` | 59,232 → 59,232 (+0.00%) | 262,144 → 262,144 (+0.00%) | 5 → 5 | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,3)` |
| fp32_onehot | `final=20:1;precommitted=14:1` | 59,350 → 59,350 (+0.00%) | 262,144 → 262,144 (+0.00%) | 5 → 5 | `L0[(3,3),(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(3,3),(3,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,3)` |
| fp32_onehot | `final=28:1;precommitted=` | 64,103 → 64,103 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,3)` |
| fp32_onehot | `final=30:1;precommitted=` | 66,644 → 66,014 (-0.95%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,5)] L5[(5,6)] T(6,6)` | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,3)] L4[(3,3)] L5[(3,5)] T(5,3)` |
| fp32_onehot | `final=34:1;precommitted=` | 67,950 → 67,872 (-0.11%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,6)] L4[(6,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense | `final=14:1;precommitted=` | 61,050 → 61,050 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(4,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(4,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,3)` |
| fp64_dense | `final=16:1;precommitted=` | 62,956 → 62,956 (+0.00%) | 131,072 → 131,072 (+0.00%) | 5 → 5 | `L0[(13,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,6)` | `L0[(13,3)] L1[(3,5)] L2[(5,6)] L3[(6,6)] T(6,3)` |
| fp64_dense | `final=20:1;precommitted=` | 65,737 → 65,585 (-0.23%) | 262,144 → 180,224 (-31.25%) | 6 → 6 | `L0[(13,3)] L1[(3,4)] L2[(4,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(13,3)] L1[(3,5)] L2[(5,4)] L3[(4,6)] L4[(6,6)] T(6,3)` |
| fp64_dense | `final=20:1;precommitted=16:1` | 65,739 → 65,739 (+0.00%) | 327,680 → 327,680 (+0.00%) | 6 → 6 | `L0[(13,3),(13,3)] L1[(3,4)] L2[(4,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(13,3),(13,3)] L1[(3,4)] L2[(4,5)] L3[(5,6)] L4[(6,6)] T(6,3)` |
| fp64_dense | `final=26:1;precommitted=` | 68,548 → 68,548 (+0.00%) | 1,572,864 → 1,572,864 (+0.00%) | 7 → 7 | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(11,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense | `final=28:1;precommitted=` | 68,664 → 68,664 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(8,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense | `final=29:1;precommitted=` | 69,054 → 69,054 (+0.00%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(16,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense | `final=30:1;precommitted=` | 69,610 → 69,562 (-0.07%) | 5,767,168 → 5,767,168 (+0.00%) | 7 → 7 | `L0[(6,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(6,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense_recursive | `final=21:1;precommitted=` | 66,333 → 66,333 (+0.00%) | 262,144 → 262,144 (+0.00%) | 6 → 6 | `L0[(11,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(11,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,3)` |
| fp64_dense_recursive | `final=23:1;precommitted=` | 73,136 → 70,820 (-3.17%) | 524,288 → 524,288 (+0.00%) | 7 → 7 | `L0[(11,3)] L1[(11,6),(3,6)] L2[(6,6)] L3[(6,6)] L4[(6,6)] L5[(6,6)] T(6,6)` | `L0[(11,3)] L1[(11,6),(3,6)] L2[(6,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense_recursive | `final=25:1;precommitted=` | 70,318 → 69,928 (-0.55%) | 1,048,576 → 1,048,576 (+0.00%) | 7 → 7 | `L0[(11,3)] L1[(13,4),(3,4)] L2[(4,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(11,3)] L1[(13,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense_recursive | `final=27:1;precommitted=` | 70,942 → 70,942 (+0.00%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(16,3)] L1[(8,4),(3,4)] L2[(4,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_dense_recursive | `final=29:1;precommitted=` | 72,666 → 71,488 (-1.62%) | 4,194,304 → 4,194,304 (+0.00%) | 7 → 7 | `L0[(16,3)] L1[(13,4),(3,4)] L2[(4,4)] L3[(4,4)] L4[(4,6)] L5[(6,6)] T(6,6)` | `L0[(16,3)] L1[(13,4),(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_onehot | `final=28:1;precommitted=` | 66,573 → 66,573 (+0.00%) | 1,048,576 → 1,048,576 (+0.00%) | 6 → 6 | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,3)] L2[(3,5)] L3[(5,6)] L4[(6,6)] T(6,3)` |
| fp64_onehot | `final=30:1;precommitted=` | 67,992 → 67,992 (+0.00%) | 2,097,152 → 2,097,152 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,3)] L2[(3,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_onehot | `final=34:1;precommitted=` | 69,610 → 69,562 (-0.07%) | 8,388,608 → 8,388,608 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| fp64_onehot | `final=35:1;precommitted=` | 69,610 → 69,562 (-0.07%) | 16,777,216 → 16,777,216 (+0.00%) | 7 → 7 | `L0[(3,3)] L1[(3,3)] L2[(3,4)] L3[(4,5)] L4[(5,6)] L5[(6,6)] T(6,6)` | `L0[(3,3)] L1[(3,4)] L2[(4,3)] L3[(3,5)] L4[(5,6)] L5[(6,6)] T(6,3)` |
| **Total (151 rows, 27 families)** | | 10,419,942 → 10,335,206 (-0.81%) | 3,762,709,696 → 3,762,160,320 (-0.01%) | 902 → 893 | | |

## Validation

All required checks were run after catalog publication and the test-fixture
adjustment. Formatting, scoped Clippy, planner library tests (87 passed; three
pre-existing ignored), schedules library tests (15 passed), all three config
integration targets (4 + 3 + 23 passed), all four requested PCS commands
(1 + 1 + 1 + 4 passed; one pre-existing manual measurement ignored), and the
catalog-security audit pass. No assertion was weakened, and no new test was disabled.

The only failing required check is `bash .remote/drift.sh`: exit 137, as
recorded above. Local no-target, external-artifact source guard, nonempty
catalog-diff, note-existence, and whitespace gates pass. Individual command
statuses and measurements are retained in the small
[validation summary](../../artifacts/floor-off-default/validation-summary.json).

Remote documentation guardrails pass. Their optional mdbook build was not
run because mdbook is not installed on the box. Scratch generation, recovery,
build, and benchmark logs have been removed; only the four linked JSON
summaries remain under `artifacts/floor-off-default/`.

The branch is not ready to pass the acceptance matrix until full two-worker
regeneration fits. A possible storage-only optimization would pack completed
suffix-frontier maps and candidate lists into boxed ordered slices, preserving
all candidates, keys, and iteration order. It remains unapplied and untested
because it exceeds the original policy/default/digest-only source scope.
