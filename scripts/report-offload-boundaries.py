#!/usr/bin/env python3
"""Render audited offload-survey results without interpolating missing cells."""
import argparse
import collections
import csv
import json
from pathlib import Path


def points(values):
    values = sorted(set(values))
    groups = []
    for value in values:
        if groups and value == groups[-1][-1] + 1:
            groups[-1].append(value)
        else:
            groups.append([value])
    return ", ".join(str(g[0]) if len(g) == 1 else f"{g[0]}–{g[-1]}" for g in groups) or "—"


def validate_record(record):
    if record["status"] != "ok":
        return
    catalog = json.loads(Path(record["metrics"]["artifact"]).read_text())
    matches = []
    for entry in catalog["rows"]:
        schedule = entry["schedule"]
        groups = schedule["root"]["params"]["groups"]["entries"]
        if len(groups) == 1 and groups[0]["profile"]["group"] == {"num_vars": record["num_vars"], "num_polynomials": 1}:
            matches.append(schedule)
    if len(matches) != 1:
        raise ValueError(f"Artifact has no unique matching scalar row: {record}")
    producers = [i for i, fold in enumerate(matches[0]["recursive_folds"]) if any(group["setup_natural_len"] is not None for group in fold["params"]["groups"]["entries"])]
    if len(producers) != int(record["metrics"]["offloads"]) or any(level > 1 for level in producers) or ",".join(map(str, producers)) != record["metrics"]["producer_levels"]:
        raise ValueError(f"Recorded offload count disagrees with artifact topology: {record}")
    return tuple(catalog["policy_digest"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("artifacts/offload-survey"))
    args = parser.parse_args()
    rows = [json.loads(p.read_text()) for p in sorted((args.output / "records").glob("*.json"))]
    policy_digests = collections.defaultdict(set)
    for row in rows:
        digest = validate_record(row)
        if digest is not None:
            group = (row["field"], row["source"], row["chunks"], row["activated_levels"])
            policy_digests[group].add(digest)
    if any(len(digests) != 1 for digests in policy_digests.values()):
        raise ValueError("A field/source/layout series mixes policy digests")
    fields = ["field", "source", "chunks", "activated_levels", "num_vars", "status", "provenance", "offloads", "producer_levels", "root_setup_fields", "first_direct_capacity", "setup_fields", "proof_bytes", "root_witness_fields", "fold_levels", "planning_secs", "elapsed_secs", "artifact", "error"]
    with (args.output / "results.tsv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for row in rows:
            result = {k: row.get(k, "") for k in fields}
            result["provenance"] = row.get("provenance", "generated")
            result.update({k: v for k, v in row.get("metrics", {}).items() if k in fields})
            result["error"] = row.get("stderr", "").strip().replace("\n", " ")
            writer.writerow(result)
    groups = collections.defaultdict(list)
    for row in rows:
        groups[(row["field"], row["source"], row["chunks"], row["activated_levels"])].append(row)
    observed_groups = len(groups)
    for field in ["fp32", "fp64", "fp128"]:
        for source in ["dense", "onehot"]:
            for w, r in [(1, 0)] + [(w, r) for r in [1, 2, 3] for w in [2, 4, 8]]:
                groups.setdefault((field, source, w, r), [])
    ordered_groups = {group: sorted((c["num_vars"], int(c["metrics"]["offloads"])) for c in cells if c["status"] == "ok") for group, cells in groups.items()}
    zero_and_positive = sum(any(count == 0 for _, count in values) and any(count > 0 for _, count in values) for values in ordered_groups.values())
    two_seen = sum(any(count == 2 for _, count in values) for values in ordered_groups.values())
    adjacent_zero = sum(any(left[1] == 0 and right[1] > 0 and right[0] == left[0]+1 for left, right in zip(values, values[1:])) for values in ordered_groups.values())
    adjacent_two = sum(any(left[1] == 1 and right[1] == 2 and right[0] == left[0]+1 for left, right in zip(values, values[1:])) for values in ordered_groups.values())
    counts = collections.Counter(row["status"] for row in rows)
    text = ["# Existing planner: setup-offload boundary survey", "",
        "Baseline: `fddf785cf1d6011b508c271d78e69958d1fde4af`. Existing recursive comparator and `RootAndFirstChildV1`; zero offloads remain eligible. No automatic root threshold is applied.", "",
        "Each cell opens one polynomial in one group, with the stock dense or one-hot source contract. W is the witness chunk count and R the number of leading levels using it. W1R0 is unchunked. One-hot source density is the stock config contract and is separate from W. Custom field/chunk combinations use the stock field config with only its witness-chunk policy changed; they are not claims that application presets already ship for those combinations.", "",
        "Every newly generated successful cell is a complete planner result independently audited by `ValidatedScheduleCatalog::try_new` and saved as a canonical `.aks` artifact. Committed scalar rows from the baseline catalogs are reused where available and marked committed_catalog in the records/TSV; their unavailable timing and size estimates are left blank. Their four source catalogs are frozen byte-for-byte in baseline_catalogs/, so retiring runtime companion artifacts does not alter this evidence; each reused record retains its original baseline artifact path. Newly generated sizes are planner estimates, not end-to-end measured proof sizes or verifier timings. Planning durations are diagnostics collected under concurrent system load and are not controlled performance benchmarks.", "",
        f"Recorded cells: **{len(rows)}** across **{observed_groups}/60** field/source/layout combinations; statuses: " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items())) + ".", "",
        f"Count coverage: **{zero_and_positive}/60** configurations have both zero and positive offload observations; **{two_seen}/60** have a two-offload observation. Adjacent 0→positive changes are observed in **{adjacent_zero}/60** configurations, and adjacent 1→2 changes in **{adjacent_two}/60**. These are local observations, not global monotonicity claims.", "",
        "The table lists tested variable counts only. A range such as 20–24 means every integer in that range was successfully planned. Separate points such as 20, 24 do not establish the behavior at 21–23. Missing cells are not interpolated. A dash in a count column means that count was not observed in a completed cell; it does not establish impossibility. An error or timeout does not establish an offload count, and the largest tested point does not establish behavior at every larger input.", "",
        "| Field | Source | Layout | 0 offloads: tested nv | 1 offload: tested nv | 2 offloads: tested nv | Errors: nv | Timeouts: nv | Missing initial-grid nv |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for (field, source, w, r), cells in sorted(groups.items(), key=lambda x:(int(x[0][0][2:]), x[0][1], x[0][2], x[0][3])):
        selected = [points(c["num_vars"] for c in cells if c["status"] == "ok" and int(c["metrics"]["offloads"]) == n) for n in range(3)]
        errors = points(c["num_vars"] for c in cells if c["status"] == "error")
        timeouts = points(c["num_vars"] for c in cells if c["status"] == "timeout")
        missing = points(set([16, 24, 32, 40]) - {c["num_vars"] for c in cells})
        text.append(f"| {field} | {source} | W{w}R{r} | {' | '.join(selected)} | {errors} | {timeouts} | {missing} |")
    text += ["", "## Observed transition brackets", "", "An arrow between neighboring integers confirms that local change. An ellipsis marks a bracket with unmeasured or unsuccessful interior points. These are observed changes within the tested range, not a guarantee about every larger input.", "", "| Field | Source | Layout | Changes in selected offload count |", "| --- | --- | --- | --- |"]
    for (field, source, w, r), completed in sorted(ordered_groups.items(), key=lambda x:(int(x[0][0][2:]), x[0][1], x[0][2], x[0][3])):
        changes = []
        for left, right in zip(completed, completed[1:]):
            if left[1] != right[1]:
                separator = "→" if right[0] == left[0]+1 else "…"
                changes.append(f"nv {left[0]}{separator}{right[0]}: {left[1]}→{right[1]}")
        text.append(f"| {field} | {source} | W{w}R{r} | {'; '.join(changes) or 'No change observed'} |")
    reversals = []
    for (field, source, w, r), cells in sorted(groups.items()):
        ok = ordered_groups[(field, source, w, r)]
        for left, right in zip(ok, ok[1:]):
            if right[1] < left[1]:
                reversals.append(f"| {field} | {source} | W{w}R{r} | {left[0]}: {left[1]} | {right[0]}: {right[1]} |")
    text += ["", "## Observed decreases in offload count", "", "A larger instance can select fewer offloads. These endpoints demonstrate that a single monotone cutoff is not an invariant of the existing comparator. Untested intermediate points remain unknown.", "", "| Field | Source | Layout | Smaller nv: offloads | Larger nv: offloads |", "| --- | --- | --- | --- | --- |"]
    text += reversals or ["| — | — | — | No decrease observed in completed cells | — |"]
    text += ["", "Full per-cell values and errors: [results.tsv](results.tsv). Canonical schedules: [schedules/](schedules/). Resumable commands, status, and diagnostics: [records/](records/).", "",
        "## Interpretation", "", "The existing comparator minimizes the padded setup envelope, then the first remaining direct-scan capacity, then the modeled proof/work score. Candidate geometry and power-of-two buckets can change the winner at neighboring sizes. Offload counts alone do not establish actual verifier latency. The adaptive refinement mode narrows observed changes only; equal counts at sampled endpoints can still hide untested islands, so the tables never interpolate them.", "", "## Scope", "", "This survey covers the unchunked layout and W2/W4/W8 at R1, R2, and R3 across all six field/source pairs. Alternative one-hot density contracts are outside this grid; the stock density is one nonzero per 256 source entries. Each row is a scalar opening; grouped openings can select a different number of offloads at the same final-group variable count. The previously observed two-offload grouped W8R2 row at final nv34 is therefore not a scalar boundary measurement.", "", "## Validation", "", "The new example passed scoped release Clippy. Repository formatting, TOML formatting, Rust file-size gates, all 179 Python script tests, crate-dependency guards, shared-field identity checks, external-artifact guards, cargo-machete, spelling, and documentation guardrails passed. The mdbook build was skipped because mdbook is unavailable. The standard generator reproduced the existing fp128 dense recursive catalog byte-for-byte. Every report render checks offload counts against serialized topology and requires one policy digest per field/source/layout series.", "", "## Reproduction", "", "Build `offload_boundary_survey` with `cargo build --release -p akita-planner --features catalog-gen --example offload_boundary_survey`, then run `scripts/survey-offload-boundaries.py --binary <path-to-example>`. Use `--nvs`, `--fields`, `--sources`, and `--layouts` to select cells; layout syntax is chunks:activated-levels. Replay the exact captured cells with `--manifest artifacts/offload-survey/results.tsv` and a fresh `--output` directory. Existing records are skipped unless `--retry` is supplied. Use a fresh output directory when planner source or policy changes. `--timeout` bounds each individual search and records timeouts separately. Run `scripts/report-offload-boundaries.py` to refresh this report.", ""]
    text += ["## Catalog migration", "", "The current single-policy catalogs are separate from this baseline capture. [catalog-migration.json](catalog-migration.json) compares all 27 former offloading rows: 25 schedules are identical; two W2R3 grouped rows have changed producer profiles; all workload shapes remain present and all eight policy digests are unchanged. Reproduce with `scripts/compare-offloading-catalogs.py`. The Book explains the configuration and setup migration.", ""]
    (args.output / "README.md").write_text("\n".join(text))
    print(f"Wrote {len(rows)} cells and {len(groups)} family/layout rows")


if __name__ == "__main__":
    main()
