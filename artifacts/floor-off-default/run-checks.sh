#!/usr/bin/env bash
# Run only through .remote/run. Keep each harness command and its exit status.
set -uo pipefail
out=artifacts/floor-off-default/checks
mkdir -p "$out"
commands=(
'cargo fmt --all -- --check'
'cargo clippy -p akita-config -p akita-planner -p akita-schedules -p akita-params -p akita-pcs --lib --tests -- -D warnings'
'cargo test --release -p akita-planner --lib'
'cargo test --release -p akita-schedules --lib'
'cargo test --release -p akita-config --test small_dense_source --test trusted_schedule_artifact --test bounded_dense_source'
'cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features --features parallel,transcript-blake2b small_dense'
'cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features --features parallel,transcript-blake2b -- --exact fp128_dense_mc'
'cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features --features parallel,transcript-blake2b -- --exact fp128_onehot_mc_catalog_resolves'
'cargo test --release -p akita-pcs --test akita_fp128_e2e --no-default-features --features parallel,transcript-blake2b -- heterogeneous::bounded_'
'cargo run --release -p akita-planner --features catalog-security --example catalog_security -- --check'
'bash .remote/drift.sh'
)
failed=0
: > "$out/status.tsv"
for i in "${!commands[@]}"; do
    command=${commands[$i]}
    echo "CHECK $i: $command"
    /usr/bin/time -v bash -c "$command" > "$out/$i.log" 2>&1 &
    pid=$!
    ticks=0
    while kill -0 "$pid" 2>/dev/null; do
        if (( ticks % 9 == 0 )); then tail -3 "$out/$i.log"; fi
        sleep 5
        ticks=$((ticks + 1))
    done
    wait "$pid"
    rc=$?
    printf '%s\t%s\t%s\n' "$i" "$rc" "$command" >> "$out/status.tsv"
    tail -28 "$out/$i.log"
    if [ "$rc" != 0 ]; then failed=1; fi
done
exit "$failed"
