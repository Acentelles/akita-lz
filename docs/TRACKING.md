# Extension seam tracker

In-repository tracker for `Extension seam:` doc comments while no tracking
issue exists (`docs/branches.md`, "Land seams on `main` first"). Each seam
comment names its entry here. An entry may later be replaced by a tracking
issue; the seam comment then names that issue instead. Record paths are
relative to the external research vault, not to this repository.

| Entry | Seam | Recorded | Review | Record |
|---|---|---|---|---|
| PRB-R3-F1 | `CommitmentConfig::terminal_quotient` (`crates/akita-config/src/lib.rs`), terminal quotient certificate selection | 2026-10-24 | `4422f56e` | Research vault (not in this repository), vault-relative `bench-logs/jolt-recursion-2p22-20261020/research-20261022/TRIED-AND-BLOCKED-20261023.md`, revision 26, row "PR B revision 3" |
| PRB-R3-F2a | `new_prover_channel_for` (`crates/akita-transcript/src/proof_stream.rs`), prover channel under a selected proof-stream version | 2026-10-24 | `4422f56e` | same registry, revision 26, row "PR B revision 3" |
| PRB-R3-F2b | `new_verifier_channel_for` (`crates/akita-transcript/src/proof_stream.rs`), verifier channel under a selected proof-stream version | 2026-10-24 | `4422f56e` | same registry, revision 26, row "PR B revision 3" |

Seam pull request against `main`: pending for all three entries. Landing
route: the stacked branch `feat/terminal-quotient-certificate-r3`, which is
not a completed `dev` landing.
