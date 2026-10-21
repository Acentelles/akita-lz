//! Compatibility re-export for jolt-akita; removed once jolt-akita ports (RP3)
//! and Akita re-pins.
//!
//! Sparse challenges live in `jolt_crypto::challenge`. Akita's own crates use
//! that module directly.
//!
//! With `labinius-challenges`, this crate also carries the LaBinius binary
//! challenge families until `jolt-crypto` owns them:
//! `BinaryChallengeProfile` / `BinaryChallengeSampler` — exact
//! parity-injective fixed- and bounded-weight support families for the
//! degree-162 and degree-486 LaBinius scalar rings.

#[cfg(feature = "labinius-challenges")]
mod binary;

#[cfg(feature = "labinius-challenges")]
pub use binary::{
    BinaryChallenge, BinaryChallengeDraw, BinaryChallengeFamily, BinaryChallengeProfile,
    BinaryChallengeSampler, BinaryChallengeTerm, BinaryScalarRing, BinarySignRule,
    INLINE_BINARY_WEIGHT,
};
pub use jolt_crypto::challenge::{Challenges, SparseChallenge, SparseChallengeConfig};
