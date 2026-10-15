#!/usr/bin/env python3
"""Compare former offloading companions with their consolidated catalogs."""
import argparse
import json
import subprocess
from pathlib import Path


FAMILIES = {
    "fp128_onehot_recursive": "fp128_onehot",
    "fp128_onehot_recursive_multi_chunk_w8r2": "fp128_onehot_multi_chunk",
    "fp128_dense_recursive": "fp128_dense",
    "fp32_dense_recursive": "fp32_dense",
    "fp64_dense_recursive": "fp64_dense",
    **{f"fp128_onehot_recursive_multi_chunk_w{w}r3":
       f"fp128_onehot_multi_chunk_w{w}r3" for w in (2, 4, 8)},
}


def row_key(schedule, *, profiles):
    groups = schedule["root"]["params"]["groups"]["entries"]
    return json.dumps([
        groups[-1]["profile"]["group"],
        [g["profile"] if profiles else g["profile"]["group"] for g in groups[:-1]],
    ], sort_keys=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default="fddf785cf1d6011b508c271d78e69958d1fde4af")
    parser.add_argument("--output", type=Path,
                        default=Path("artifacts/offload-survey/catalog-migration.json"))
    args = parser.parse_args()
    comparisons = []
    for old, new in FAMILIES.items():
        before = json.loads(subprocess.check_output([
            "git", "show", f"{args.baseline}:artifacts/schedules/{old}.aks",
        ]))
        after = json.loads(Path(f"artifacts/schedules/{new}.aks").read_text())
        current = {row_key(r["schedule"], profiles=True): r["schedule"]
                   for r in after["rows"]}
        workloads = {row_key(r["schedule"], profiles=False) for r in after["rows"]}
        equal = changed = changed_profiles = 0
        for row in before["rows"]:
            schedule = row["schedule"]
            key = row_key(schedule, profiles=True)
            if key in current:
                equal += current[key] == schedule
                changed += current[key] != schedule
            elif row_key(schedule, profiles=False) in workloads:
                changed_profiles += 1
            else:
                raise ValueError(f"Missing former offloading workload: {old}: {key}")
        if before["policy_digest"] != after["policy_digest"]:
            raise ValueError(f"Surviving policy identity changed: {old}")
        comparisons.append({
            "baseline_family": old, "current_family": new,
            "baseline_rows": len(before["rows"]), "identical_schedules": equal,
            "changed_schedules_with_identical_input_profiles": changed,
            "rows_with_changed_producer_profiles": changed_profiles,
            "missing_workload_shapes": 0, "policy_digest_unchanged": True,
        })
    result = {"baseline_commit": args.baseline, "families": comparisons}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Compared {sum(r['baseline_rows'] for r in comparisons)} former rows; "
          f"{sum(r['identical_schedules'] for r in comparisons)} identical, "
          f"{sum(r['rows_with_changed_producer_profiles'] for r in comparisons)} "
          "with changed producer profiles; no missing workload shapes.")


if __name__ == "__main__":
    main()
