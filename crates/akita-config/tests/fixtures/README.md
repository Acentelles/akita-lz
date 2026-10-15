# Scanning-alphabet regression fixtures

These fixed nv14 rows preserve the B-scanning regression geometry from
`fddf785cf1d6011b508c271d78e69958d1fde4af`. They deliberately exercise the
rank boundary between the honest base-four alphabet and the certified
base-eight scanning alphabet. Changing the optimizer's preferred small layout
must not erase that boundary from the test.

Both catalogs use the sole current policy identity. They are explicit protocol
fixtures, not claims about the planner's current optimum. Tests load them through
`TrustedScheduleCatalog` and require independent admission before mutating the
opening basis or testing malformed bounds. No alternate planner mode is used.

## Updating policy identities

Keep these rows fixed when optimizer preferences change. If only the policy
identity changes, refresh each fixture's `policy_digest` from the corresponding
current `artifacts/schedules/fp32_onehot.aks`, `fp64_onehot.aks`, or
`fp128_onehot.aks` envelope. Do not replace its `rows` with the new optimum.
For example, from the repository root:

```sh
python3 - <<'PYTHON'
import json
import re
from pathlib import Path
for fixture in Path("crates/akita-config/tests/fixtures").glob("*_scanning.aks"):
    data = json.loads(fixture.read_text())
    current = json.loads(Path(f"artifacts/schedules/{data['family_name']}.aks").read_text())
    # Preserve the canonical compact schedule rows; replace only the header array.
    digest = json.dumps(current["policy_digest"], indent=2).replace("\n", "\n  ")
    text = re.sub(r'("policy_digest": )\[.*?\]',
                  lambda match: match[1] + digest, fixture.read_text(),
                  count=1, flags=re.S)
    fixture.write_text(text)
PYTHON
cargo test -p akita-config --test bounded_dense_source
```

Admission is mandatory after rebinding. If policy changes also affect security,
layout, or transcript constraints, a digest update alone is insufficient: rebuild
an independently admitted row retaining the scanning boundary exercised by the
tests. Never bypass admission or weaken those assertions to refresh a fixture.
