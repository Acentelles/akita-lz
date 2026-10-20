#![no_main]

use akita_error::AkitaError;
use jolt_algebra::{CanonicalBytes, Prime128Offset275 as F, Ring};
use jolt_sumcheck::{
    verify_single, SingleFormat, SingleInstance, SumcheckSite, VerifierRoundContext,
};
use jolt_transcript::{Blake2b512, Channel, ProtocolId, SiteId, VerifierTranscript};
use libfuzzer_sys::fuzz_target;

const PROTOCOL: ProtocolId = ProtocolId::new::<Blake2b512>("akita-fuzz/sumcheck-rounds");

struct FuzzVerifierChannel<'proof> {
    state: VerifierTranscript<'proof, Blake2b512>,
    challenges: usize,
}

impl<'proof> VerifierRoundContext<'proof, F> for FuzzVerifierChannel<'proof> {
    type Sponge = Blake2b512;
    type Error = AkitaError;

    fn transcript_mut(&mut self) -> &mut VerifierTranscript<'proof, Blake2b512> {
        &mut self.state
    }

    fn site(&self, site: SumcheckSite) -> Option<SiteId> {
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(&site.round.to_le_bytes());
        bytes[4..8].copy_from_slice(&(site.role as u32).to_le_bytes());
        Some(SiteId(bytes))
    }

    fn challenge(&mut self, site: SumcheckSite) -> Result<F, AkitaError> {
        if let Some(site) = self.site(site) {
            self.state.site(site);
        }
        self.challenges += 1;
        Ok(self.state.challenge())
    }
}

fuzz_target!(|data: &[u8]| {
    let [rounds, degree, claim, data @ ..] = data else {
        return;
    };
    let num_rounds = usize::from(rounds % 5);
    let degree_bound = usize::from(degree % 5);
    let mut channel = FuzzVerifierChannel {
        state: VerifierTranscript::new(&PROTOCOL, b"fixture", data),
        challenges: 0,
    };
    let instance = SingleInstance {
        input_claim: F::from_u64(u64::from(*claim)),
        rounds: num_rounds,
        degree: degree_bound,
        format: SingleFormat::Compressed,
        invocation: 0,
    };
    let result = verify_single::<F, F, AkitaError, _>(&instance, &mut channel);
    assert!(channel.challenges <= num_rounds);
    let round_bytes = degree_bound * F::NUM_BYTES;
    let complete_input_rounds = if round_bytes == 0 {
        0
    } else {
        (data.len() / round_bytes).min(num_rounds)
    };
    assert!(channel.challenges <= complete_input_rounds);
    if degree_bound == 0 {
        assert!(result.is_err());
    }
    if let Ok(replay) = result {
        assert_eq!(replay.challenges.len(), num_rounds);
        assert_eq!(channel.challenges, num_rounds);
        let expected_bytes = num_rounds * round_bytes;
        assert_eq!(channel.state.finish().is_ok(), data.len() == expected_bytes);
    }
});
