"""Remote-only driver for the ignored arity-19 measurement test."""
from pathlib import Path
import json
import os
import re
import subprocess

ROOT = Path("/tmp/akita-root-basis-report")
NOTE = Path("docs/notes/root-basis-5-dense-small.md")
COMMAND = ["cargo", "test", "--release", "-p", "akita-pcs", "--test", "akita_fp128_e2e",
           "--no-default-features", "--features", "parallel,transcript-blake2b",
           "small_dense_measurement", "--", "--ignored", "--nocapture"]


def stage1_seconds(path):
    data = json.loads(path.read_text())
    events = data if isinstance(data, list) else data["traceEvents"]
    stacks = {}
    elapsed = 0.0
    for event in events:
        key = (event.get("pid"), event.get("tid"))
        if event.get("ph") == "B":
            stacks.setdefault(key, []).append(event)
        elif event.get("ph") == "E":
            start = stacks[key].pop()
            if start.get("name") == "stage1_sumcheck":
                elapsed += event["ts"] - start["ts"]
        elif event.get("ph") == "X" and event.get("name") == "stage1_sumcheck":
            elapsed += event["dur"]
    if elapsed <= 0:
        raise ValueError("no stage1_sumcheck timing found")
    return elapsed / 1_000_000


def main():
    rows = []
    for k, variant in ((0, 0), (5, 1), (5, 3), (6, 1), (6, 3)):
        env = dict(os.environ, AKITA_MEASURE_K=str(k), AKITA_MEASURE_VARIANT=str(variant),
                   RAYON_NUM_THREADS="8", CARGO_BUILD_JOBS="1")
        result = subprocess.run(COMMAND, env=env, text=True, capture_output=True)
        log = ROOT / f"measure_k{k}_v{variant}.log"
        log.write_text(result.stdout + result.stderr)
        result.check_returncode()
        line = next(line for line in result.stdout.splitlines() if line.startswith("MEASURE "))
        fields = dict(re.findall(r"(\w+)=([^ ]+)", line))
        commit, prove, verify = (float(fields[name]) for name in ("commit_s", "prove_s", "verify_s"))
        stage1 = stage1_seconds(ROOT / f"k{k}_v{variant}.json")
        rows.append(f"| {k} | V{variant} | {fields['bytes']} | {commit:.3f} | {prove:.3f} | "
                    f"{commit + prove:.3f} | {verify:.4f} | {stage1:.3f} | "
                    f"{100 * stage1 / prove:.1f}% | {100 * stage1 / (commit + prove):.1f}% |")
        print(line, f"stage1_s={stage1:.6f}", flush=True)
    text = NOTE.read_text() + "\n\n### Arity-19 measured commit/open/verify\n\n"
    text += "One run per variant on the 8-vCPU / 16-GB Boat box, release build, eight Rayon\n"
    text += "threads, Blake2b transcript. Timers exclude setup construction and the independent\n"
    text += "evaluation oracle. Commit and opening are separate timers; prover total is their sum.\n"
    text += "Stage 1 is the sum of inclusive `stage1_sumcheck` span durations across nonterminal\n"
    text += "levels, collected with INFO tracing during the same run. It includes the range tree\n"
    text += "and any fused norm proof. These are single samples, not confidence intervals.\n\n"
    text += "| K | Variant | Real proof bytes | Commit s | Open s | Prover total s | Verify s | Stage 1 s | Stage 1 / open | Stage 1 / total |\n"
    text += "|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|\n" + "\n".join(rows) + "\n"
    NOTE.write_text(text)


if __name__ == "__main__":
    main()
