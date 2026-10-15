#!/usr/bin/env python3
"""Run resumable, bounded independent cells of the existing recursive planner."""
import argparse
import concurrent.futures
import csv
import itertools
import json
import subprocess
import time
from pathlib import Path

COLUMNS = ["family", "chunks", "activated_levels", "num_vars", "offloads", "producer_levels", "root_setup_fields", "first_direct_capacity", "setup_fields", "proof_bytes", "root_witness_fields", "fold_levels", "planning_secs", "artifact"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("artifacts/offload-survey"))
    parser.add_argument("--nvs", type=int, nargs="+", default=[16, 24, 32, 40])
    parser.add_argument("--jobs", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--layouts", nargs="+", default=["1:0", "2:1", "4:1", "8:1", "2:2", "4:2", "8:2", "2:3", "4:3", "8:3"])
    parser.add_argument("--fields", nargs="+", default=["fp32", "fp64", "fp128"])
    parser.add_argument("--sources", nargs="+", default=["dense", "onehot"])
    parser.add_argument("--retry", action="store_true")
    parser.add_argument("--manifest", type=Path, help="Replay field/source/layout/nv cells from a results TSV")
    parser.add_argument("--cells", nargs="+", help="Explicit field:source:chunks:levels:nv cells")
    parser.add_argument("--refine", action="store_true", help="Refine observed count changes; wait for the coarse grid")
    parser.add_argument("--max-refinements", type=int, default=120)
    args = parser.parse_args()
    if args.jobs < 1 or args.timeout < 1:
        parser.error("jobs and timeout must be positive")
    records = args.output / "records"
    records.mkdir(parents=True, exist_ok=True)
    artifact_dir = args.output / "schedules"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    layouts = [tuple(map(int, layout.split(":"))) for layout in args.layouts]
    cells = [(f, s, w, r, n) for n, (w, r), s, f in itertools.product(args.nvs, layouts, args.sources, args.fields)]

    if args.cells:
        cells = []
        for cell in args.cells:
            field, source, chunks, levels, nv = cell.split(":")
            cells.append((field, source, int(chunks), int(levels), int(nv)))

    if args.manifest:
        if args.cells or args.refine:
            parser.error("manifest cannot be combined with cells or refine")
        with args.manifest.open() as stream:
            cells = [(row["field"], row["source"], int(row["chunks"]), int(row["activated_levels"]), int(row["num_vars"])) for row in csv.DictReader(stream, delimiter="\t")]
    cells = list(dict.fromkeys(cells))

    def run(cell):
        field, source, chunks, levels, nv = cell
        name = f"{field}_{source}_w{chunks}r{levels}_nv{nv}"
        destination = records / f"{name}.json"
        if destination.exists() and not args.retry:
            return name, "cached"
        cmd = [str(args.binary.resolve()), field, source, str(chunks), str(levels), str(nv), str(artifact_dir)]
        start = time.monotonic()
        data = dict(field=field, source=source, chunks=chunks, activated_levels=levels, num_vars=nv, command=cmd, timeout_secs=args.timeout)
        try:
            completed = subprocess.run(cmd, text=True, capture_output=True, timeout=args.timeout)
            data.update(returncode=completed.returncode, stderr=completed.stderr, elapsed_secs=time.monotonic()-start)
            if completed.returncode == 0:
                values = completed.stdout.strip().split("\t")
                if len(values) != len(COLUMNS):
                    raise ValueError(f"unexpected survey output: {completed.stdout!r}")
                data.update(status="ok", metrics=dict(zip(COLUMNS, values)))
            else:
                data.update(status="error", stdout=completed.stdout)
        except subprocess.TimeoutExpired:
            data.update(status="timeout", elapsed_secs=time.monotonic()-start)
        temporary = destination.with_suffix(".tmp")
        temporary.write_text(json.dumps(data, indent=2) + "\n")
        temporary.replace(destination)
        return name, data["status"]

    if not args.refine:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = [executor.submit(run, cell) for cell in cells]
            for index, future in enumerate(concurrent.futures.as_completed(futures), 1):
                name, status = future.result()
                print(f"{index}/{len(cells)} {name} {status}", flush=True)
        return

    def refinement_choices(active_groups):
        observed = {}
        for record in records.glob("*.json"):
            data = json.loads(record.read_text())
            group = (data["field"], data["source"], data["chunks"], data["activated_levels"])
            observed.setdefault(group, {})[data["num_vars"]] = data
        choices = []
        for group, values in observed.items():
            if group in active_groups or group[0] not in args.fields or group[1] not in args.sources or group[2:] not in layouts:
                continue
            successful = sorted((n, int(data["metrics"]["offloads"])) for n, data in values.items() if data["status"] == "ok")
            for left, right in zip(successful, successful[1:]):
                if left[1] == right[1] or right[0] - left[0] <= 1:
                    continue
                available = [n for n in range(left[0] + 1, right[0]) if n not in values and n not in args.nvs]
                if available:
                    middle = min(available, key=lambda n: (abs(2*n-left[0]-right[0]), n))
                    choices.append(((min(left[1], right[1]), -(right[0]-left[0])), group + (middle,)))
        coarse_done = all(n in observed.get((f, source, w, r), {}) for f in args.fields for source in args.sources for w, r in layouts for n in args.nvs)
        return sorted(choices), coarse_done

    started = completed = 0
    active = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
        while True:
            groups = {cell[:4] for cell in active.values()}
            choices, coarse_done = refinement_choices(groups)
            for _, cell in choices:
                if len(active) >= args.jobs or started >= args.max_refinements:
                    break
                if cell[:4] in groups:
                    continue
                groups.add(cell[:4])
                active[executor.submit(run, cell)] = cell
                started += 1
            if not active:
                if started >= args.max_refinements:
                    print("Refinement cell budget reached", flush=True)
                    break
                if coarse_done:
                    print("No remaining observed changes with an untested interior", flush=True)
                    break
                time.sleep(20)
                continue
            done, _ = concurrent.futures.wait(active, timeout=20, return_when=concurrent.futures.FIRST_COMPLETED)
            for future in done:
                active.pop(future)
                name, status = future.result()
                completed += 1
                print(f"{completed}/{args.max_refinements} {name} {status}", flush=True)


if __name__ == "__main__":
    main()
