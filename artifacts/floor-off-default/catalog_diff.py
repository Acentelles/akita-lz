"""Render the floor-off audit from tracked baseline and regenerated artifacts.
Run from the checkout root with Python; this only reads data and writes reports.
"""
import csv
import io
import json
import pathlib
import subprocess

BASE = '4bdbd42fa806a3286884d4450d2b95b58205505d'
OUT = pathlib.Path('artifacts/floor-off-default')


def read(path, baseline=False):
    if baseline:
        return subprocess.check_output(['git', 'show', f'{BASE}:{path}'], text=True)
    return pathlib.Path(path).read_text()


def snapshot(baseline):
    return {(r['family'], r['logical_key']): r for r in csv.DictReader(
        io.StringIO(read('artifacts/schedule-catalog.tsv', baseline)), delimiter='\t')}


def schedules(families, baseline):
    result = {}
    # The generator distinguishes the heterogeneous bounded producer from the
    # one-hot producer at the same logical shape. Match its exact frozen profile.
    bounded = json.loads(read('artifacts/schedules/fp128_dense_bounded.aks', baseline))
    bounded_profiles = [r['schedule']['root']['params']['groups']['entries'][-1]['profile']
                        for r in bounded['rows']]
    for family in families:
        for row in json.loads(read(f'artifacts/schedules/{family}.aks', baseline))['rows']:
            s = row['schedule']
            groups = s['root']['params']['groups']['entries']
            def layout(g):
                p = g['profile']['group']
                return f"{p['num_vars']}:{p['num_polynomials']}"
            key = f"final={layout(groups[-1])};precommitted={','.join(map(layout, groups[:-1]))}"
            if family == 'fp128_onehot' and key == 'final=16:1;precommitted=14:1':
                if groups[0]['profile'] in bounded_profiles:
                    key += ';producer_contracts=balanced(bound=65)'
            if (family, key) in result:
                raise ValueError(f'Ambiguous artifact row: {family} {key}')
            parts = []
            for level, fold in enumerate([s['root'], *s['recursive_folds']]):
                pairs = [f"({g['profile']['inner']['digits']['log_basis']},{g['opening']['log_basis_open']})"
                         for g in fold['params']['groups']['entries']]
                parts.append(f"L{level}" + '[' + ','.join(pairs) + ']')
            t = s['terminal']
            parts.append(f"T({t['inner']['digits']['log_basis']},{t['fold']['log_basis']})")
            result[family, key] = ' '.join(parts)
    return result


def delta(b, a):
    return f'{int(b):,} → {int(a):,} ({(int(a)/int(b)-1)*100:+.2f}%)'


before, after = snapshot(True), snapshot(False)
families = sorted({f for f, k in before.keys() | after.keys()})
bases_before, bases_after = schedules(families, True), schedules(families, False)
if before.keys() != after.keys():
    raise ValueError(f'Catalog row membership changed: {before.keys() ^ after.keys()}')
rows = []
for key in sorted(before):
    b, a = before[key], after[key]
    rows.append(dict(family=key[0], logical_key=key[1], before=b, after=a,
                     bases_before=bases_before[key], bases_after=bases_after[key]))
OUT.joinpath('catalog-diff.json').write_text(json.dumps(rows, indent=2) + '\n')
lines = ['| Family | Logical key | Modeled proof bytes: before → after (%) | Setup fields: before → after (%) | Fold levels: before → after | Bases before | Bases after |',
         '| --- | --- | ---: | ---: | ---: | --- | --- |']
for r in rows:
    b, a = r['before'], r['after']
    lines.append(f"| {r['family']} | `{r['logical_key']}` | {delta(b['proof_bytes'], a['proof_bytes'])} | {delta(b['setup_fields'], a['setup_fields'])} | {b['fold_levels']} → {a['fold_levels']} | `{r['bases_before']}` | `{r['bases_after']}` |")
totals = {field: [sum(int(r[side][field]) for r in rows) for side in ('before', 'after')]
          for field in ('proof_bytes', 'setup_fields', 'fold_levels')}
lines.append(f"| **Total ({len(rows)} rows, {len(families)} families)** | | {delta(*totals['proof_bytes'])} | {delta(*totals['setup_fields'])} | {' → '.join(map(str,totals['fold_levels']))} | | |")
OUT.joinpath('catalog-table.md').write_text('\n'.join(lines) + '\n')

cases = [
 ('dense_fp32:30:1', 'fp32_dense', 'final=30:1;precommitted='),
 ('onehot_fp32:34:1', 'fp32_onehot', 'final=34:1;precommitted='),
 ('dense_fp64:29:1', 'fp64_dense', 'final=29:1;precommitted='),
 ('onehot_fp64:35:1', 'fp64_onehot', 'final=35:1;precommitted='),
 ('dense_fp128:28:1', 'fp128_dense', 'final=28:1;precommitted='),
 ('onehot_fp128:36:1:direct', 'fp128_onehot', 'final=36:1;precommitted='),
 ('onehot_fp128:36:1:recursive', 'fp128_onehot_recursive', 'final=36:1;precommitted='),
 ('onehot_fp128_multi_group:34:4:direct', 'fp128_onehot', 'final=34:2;precommitted=16:1,16:1'),
 ('onehot_fp128_multi_group_recursive:34:4:recursive', 'fp128_onehot_recursive', 'final=34:2;precommitted=16:1,16:1'),
 ('onehot_fp128_multi_group_recursive_multi_chunk_w8r2:32:4:recursive', 'fp128_onehot_recursive_multi_chunk_w8r2', 'final=32:2;precommitted=16:1,16:1'),
 ('onehot_fp128_multi_chunk_w2r2:32:1', 'fp128_onehot_multi_chunk_w2r2', 'final=32:1;precommitted='),
 ('onehot_fp128_multi_chunk_w4r2:32:1', 'fp128_onehot_multi_chunk_w4r2', 'final=32:1;precommitted='),
 ('onehot_fp128_multi_chunk_w8r2:32:1', 'fp128_onehot_multi_chunk', 'final=32:1;precommitted='),
]
lines = ['| Profile case | Catalog family / logical key | Modeled bytes: before → after (%) | Bases before | Bases after |',
         '| --- | --- | ---: | --- | --- |']
for case, family, key in cases:
    index = family, key
    lines.append(f"| `{case}` | `{family}` / `{key}` | {delta(before[index]['proof_bytes'], after[index]['proof_bytes'])} | `{bases_before[index]}` | `{bases_after[index]}` |")
OUT.joinpath('profile-catalog-table.md').write_text('\n'.join(lines) + '\n')
OUT.joinpath('cases.txt').write_text('\n'.join(c[0] for c in cases) + '\n')
print(json.dumps(totals))
