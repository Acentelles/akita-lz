//! Proof-stream grammar version named in the protocol identifier.

/// Whether this build proves and verifies the terminal quotient certificate.
///
/// Every Akita crate that prices, sends or checks the certificate asserts at
/// compile time that its own `terminal-quotient` feature agrees with this
/// value, so a build graph cannot change the identifier without also changing
/// the producers and consumers of the grammar it names.
pub const TERMINAL_QUOTIENT: bool = cfg!(feature = "terminal-quotient");

/// Grammar version segment of the proof-stream protocol identifier.
///
/// The terminal quotient certificate changes the terminal grammar, so its
/// proofs run under a separate identifier (a different initial sponge state).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofStreamVersion {
    /// Exact terminal ring relations: `v7`.
    V7,
    /// Terminal quotient certificate: `v7-terminal-quotient`.
    V7TerminalQuotient,
}

impl ProofStreamVersion {
    /// The version this build proves and verifies.
    pub const ACTIVE: Self = if TERMINAL_QUOTIENT {
        Self::V7TerminalQuotient
    } else {
        Self::V7
    };

    /// Segment written into the protocol identifier.
    #[must_use]
    pub const fn segment(self) -> &'static str {
        match self {
            Self::V7 => "v7",
            Self::V7TerminalQuotient => "v7-terminal-quotient",
        }
    }
}

#[cfg(feature = "test-support")]
thread_local! {
    static VERSION_OVERRIDE: std::cell::Cell<Option<ProofStreamVersion>> =
        const { std::cell::Cell::new(None) };
}

/// The version used by channels constructed on this thread.
#[cfg(not(feature = "test-support"))]
pub(crate) const fn channel_version() -> ProofStreamVersion {
    ProofStreamVersion::ACTIVE
}

/// The version used by channels constructed on this thread.
#[cfg(feature = "test-support")]
pub(crate) fn channel_version() -> ProofStreamVersion {
    VERSION_OVERRIDE
        .with(std::cell::Cell::get)
        .unwrap_or(ProofStreamVersion::ACTIVE)
}

/// Run `body` with every channel constructed on this thread bound to
/// `version` instead of [`ProofStreamVersion::ACTIVE`] (test support only).
///
/// Cross-version tests use it to feed one version's proof to a verifier whose
/// only difference is the version domain.
#[cfg(feature = "test-support")]
pub fn with_proof_stream_version<R>(version: ProofStreamVersion, body: impl FnOnce() -> R) -> R {
    struct Restore(Option<ProofStreamVersion>);
    impl Drop for Restore {
        fn drop(&mut self) {
            VERSION_OVERRIDE.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(VERSION_OVERRIDE.with(|cell| cell.replace(Some(version))));
    body()
}
