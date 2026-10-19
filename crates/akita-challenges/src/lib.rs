//! Compatibility re-export for jolt-akita; removed once jolt-akita ports (RP3)
//! and Akita re-pins.
//!
//! Sparse challenges live in `jolt_crypto::challenge`. Akita's own crates use
//! that module directly.

pub use jolt_crypto::challenge::{Challenges, SparseChallenge, SparseChallengeConfig};
