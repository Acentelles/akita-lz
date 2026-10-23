//! Proof-stream grammar version named in the protocol identifier.

use akita_error::AkitaError;

/// Whether the terminal quotient certificate is compiled into this build.
///
/// Compiling it in does not select it: a configuration selects it at runtime
/// (`CommitmentConfig::terminal_quotient`). Every Akita crate that prices,
/// sends or checks the certificate asserts at compile time that its own
/// `terminal-quotient` feature agrees with this value, so a build cannot carry
/// the certificate's identifier without its producers and consumers.
pub const TERMINAL_QUOTIENT: bool = cfg!(feature = "terminal-quotient");

/// Grammar version segment of the proof-stream protocol identifier.
///
/// The terminal quotient certificate changes the terminal grammar, so its
/// proofs run under a separate identifier (a different initial sponge state).
/// The prover and verifier choose the version and the grammar from the same
/// runtime selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofStreamVersion {
    /// Exact terminal ring relations: `v7`.
    V7,
    /// Terminal quotient certificate: `v7-terminal-quotient`.
    #[cfg(feature = "terminal-quotient")]
    V7TerminalQuotient,
}

impl ProofStreamVersion {
    /// The version for a configuration's terminal quotient selection.
    ///
    /// # Errors
    ///
    /// Returns [`AkitaError::UnsupportedSchedule`] when the certificate is
    /// selected but not compiled into this build.
    pub fn select(terminal_quotient: bool) -> Result<Self, AkitaError> {
        if !terminal_quotient {
            return Ok(Self::V7);
        }
        #[cfg(feature = "terminal-quotient")]
        {
            Ok(Self::V7TerminalQuotient)
        }
        #[cfg(not(feature = "terminal-quotient"))]
        {
            Err(AkitaError::UnsupportedSchedule(
                "terminal quotient certificate is not compiled into this build".into(),
            ))
        }
    }

    /// Segment written into the protocol identifier.
    #[must_use]
    pub const fn segment(self) -> &'static str {
        match self {
            Self::V7 => "v7",
            #[cfg(feature = "terminal-quotient")]
            Self::V7TerminalQuotient => "v7-terminal-quotient",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_names_the_version() {
        assert_eq!(
            ProofStreamVersion::select(false).unwrap(),
            ProofStreamVersion::V7
        );
        assert_eq!(ProofStreamVersion::V7.segment(), "v7");
        #[cfg(feature = "terminal-quotient")]
        assert_eq!(
            ProofStreamVersion::select(true).unwrap(),
            ProofStreamVersion::V7TerminalQuotient
        );
        #[cfg(not(feature = "terminal-quotient"))]
        assert!(matches!(
            ProofStreamVersion::select(true),
            Err(AkitaError::UnsupportedSchedule(_))
        ));
    }
}
