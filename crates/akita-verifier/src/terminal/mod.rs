//! Direct terminal checks over the revealed terminal response.

pub(crate) mod direct;
mod ntt;
#[cfg(feature = "terminal-quotient")]
pub(crate) mod quotient;
