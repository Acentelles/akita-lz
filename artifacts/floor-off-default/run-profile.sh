#!/usr/bin/env bash
# Run only through .remote/run, after building the profile binary.
set -uo pipefail
out="$PWD/artifacts/floor-off-default"
export AKITA_PROFILE_PROVE_THREADS=8 AKITA_PROFILE_VERIFY_THREADS=8
export AKITA_PROFILE_TRACE=0 AKITA_PROFILE_SPAN_CLOSES=0 AKITA_PROFILE_LOG=info AKITA_PROFILE_ANSI=0
export CARGO_NET_GIT_FETCH_WITH_CLI=true
args=()
while IFS= read -r case; do args+=(--case "$case"); done < "$out/cases.txt"
python3 scripts/profile_bench_report.py run \
    --binary "$PWD/target/release/examples/profile" \
    --baseline-binary /home/user/akita-baseline/target/release/examples/profile \
    --baseline-output-dir "$out/base" \
    --runs 3 --warmups 1 --output-dir "$out/head" "${args[@]}" \
    > "$out/profile-run.log" 2>&1 &
pid=$!
ticks=0
while kill -0 "$pid" 2>/dev/null; do
    if (( ticks % 9 == 0 )); then
        date -u
        tail -4 "$out/profile-run.log"
        find "$out/head" "$out/base" -name summary.json -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -2
        ps -eo pid,ppid,etime,rss,args | grep '/examples/profile' | grep -v grep | head -3
    fi
    sleep 5
    ticks=$((ticks + 1))
done
wait "$pid"
rc=$?
echo "$rc" > "$out/profile-exit-code.txt"
python3 scripts/profile_bench_report.py render "$out/head/summary.json" \
    --main-baseline-dir "$out/base" --compact > "$out/ci-comparison.md"
python3 scripts/profile_bench_report.py render "$out/head/summary.json" \
    --main-baseline-dir "$out/base" > "$out/profile-report.md"
cat "$out/ci-comparison.md"
exit "$rc"
