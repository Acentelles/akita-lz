# Extension seam tracker

In-repository tracker for `Extension seam:` doc comments while no tracking
issue exists (`docs/branches.md`, "Land seams on `main` first"). Each seam
comment names its entry here. An entry may later be replaced by a tracking
issue; the seam comment then names that issue instead.

| Entry | Seam | Recorded | Review |
|---|---|---|---|
| PRB-R3-F1 | `CommitmentConfig::terminal_quotient` (`crates/akita-config/src/lib.rs`), terminal quotient certificate selection | 2026-10-24 | internal review, 24 Oct 2026 |
| PRB-R3-F2a | `new_prover_channel_for` (`crates/akita-transcript/src/proof_stream.rs`), prover channel under a selected proof-stream version | 2026-10-24 | internal review, 24 Oct 2026 |
| PRB-R3-F2b | `new_verifier_channel_for` (`crates/akita-transcript/src/proof_stream.rs`), verifier channel under a selected proof-stream version | 2026-10-24 | internal review, 24 Oct 2026 |

Seam pull request against `main`: pending for all three entries. Landing
route: the stacked branch `feat/terminal-quotient-certificate-r3`, which is
not a completed `dev` landing.
