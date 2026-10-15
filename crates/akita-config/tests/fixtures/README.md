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
