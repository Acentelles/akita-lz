"""Run the report example remotely, preserving all variant artifacts under /tmp."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess

BINARY = Path("target/release/examples/root_basis_report")
LOGS = Path("/tmp/akita-root-basis-report")
NOTE = Path("docs/notes/root-basis-5-dense-small.md")
ARITIES = (13, 16, 19, 20, 21, 24, 29)


def run(task):
    k, variant, arity = task
    result = subprocess.run(
        [str(BINARY), str(k), str(variant), str(arity)],
        text=True, capture_output=True,
    )
    (LOGS / f"k{k}_v{variant}_n{arity}.log").write_text(result.stdout + result.stderr)
    result.check_returncode()
    row = next(line for line in result.stdout.splitlines() if line.startswith("|"))
    print(f"completed K={k} V{variant} arity={arity}", flush=True)
    return row


def main():
    LOGS.mkdir(exist_ok=True)
    tasks = [(0, 0, n) for n in ARITIES]
    tasks += [(k, v, n) for k in (5, 6) for v in range(1, 6) for n in ARITIES]
    with ThreadPoolExecutor(max_workers=4) as pool:
        rows = list(pool.map(run, tasks))
    text = NOTE.read_text().replace("<!-- BYTE_TABLE -->", "\n".join(rows))
    text += "\n\n### Existing full-field catalog, floor-off comparison\n\n"
    text += "| Arity | Floor on bytes | Floor off bytes | Delta |\n|---:|---:|---:|---:|\n"
    for n in (16, 24):
        row = run((0, 2, n))
        off = int(row.split("|")[4])
        on = int(rows[ARITIES.index(n)].split("|")[4])
        text += f"| {n} | {on} | {off} | {off - on:+d} ({100 * (off / on - 1):+.2f}%) |\n"
    text += "\n### Isolated planner wall time\n\n"
    text += "Complete DenseSmall<6> 17-key catalogs (arities 13..29), root 5, sequential\n"
    text += "processes with serial row planning, no compilation included. Floor on runs first.\n\n"
    text += "| Floor | Seconds |\n|---|---:|\n"
    for v, label in ((4, "on"), (3, "off")):
        result = subprocess.run([str(BINARY), "6", str(v), "0"], text=True,
                                capture_output=True, check=True)
        (LOGS / f"catalog_v{v}.log").write_text(result.stdout + result.stderr)
        seconds = next(line.split("=", 1)[1] for line in result.stdout.splitlines()
                       if line.startswith("CATALOG_SECONDS="))
        text += f"| {label} | {seconds} |\n"
    NOTE.write_text(text)


if __name__ == "__main__":
    main()
