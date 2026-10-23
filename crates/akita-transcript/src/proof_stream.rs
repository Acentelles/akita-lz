//! Spongefish state construction and canonical Akita message codecs.

use akita_error::{
    narrowing::{usize_to_u32, usize_to_u64},
    AkitaError,
};
use jolt_field::{CanonicalEncoding, ExtField, Field};
use spongefish::{
    protocol_id, DomainSeparator, DuplexSpongeInterface, Encoding, NargDeserialize, ProverState,
    VerificationError, WithoutInstance,
};
use std::marker::PhantomData;

use crate::{ProofStreamVersion, TranscriptSponge};

mod nonce;
pub use nonce::{nonce_encoded_len, nonce_max_bytes, NonceAtom};
mod channel;
mod sampling;
pub use channel::ProofChannel;
pub use sampling::{
    field_challenge_bytes, field_sampling_is_certified, prover_field_challenge,
    verifier_field_challenge, FIELD_CHALLENGE_BYTES, FIELD_SAMPLING_QUERY_LIMIT,
};
mod verifier;
pub use verifier::VerifierChannel;
mod site;

/// Proof channel and proof-stream format version.
pub const PROTOCOL_VERSION: u32 = 7;

/// Domain tag stored in every diagnostic context record.
pub const CONTEXT_DOMAIN: [u8; 32] = *b"akita-pcs/native-context/v7\0\0\0\0\0";

/// Stable family identifier for standard and batched sumcheck sites.
pub const SITE_FAMILY_SUMCHECK: u32 = 1;

/// Stable family identifier for extension-opening reduction messages.
pub const SITE_FAMILY_EXTENSION_OPENING_REDUCTION: u32 = 2;

/// Stable family identifier for stage-2 terminal claims.
pub const SITE_FAMILY_STAGE2: u32 = 3;

/// Stable family identifier for stage-1 late oracle claims.
pub const SITE_FAMILY_STAGE1: u32 = 4;

/// Stable family identifier for physical-L2 proof values.
pub const SITE_FAMILY_PHYSICAL_L2: u32 = 5;

/// Stable family identifier for recursive setup-product stage 3.
pub const SITE_FAMILY_STAGE3: u32 = 6;

/// Stable family identifier for indexed sparse fold-challenge roots.
pub const SITE_FAMILY_FOLD_CHALLENGE: u32 = 7;

/// Stable family identifier for ring-relation opening payloads.
pub const SITE_FAMILY_OPENING_PAYLOAD: u32 = 8;

/// Stable family identifier for derived fold-opening values.
pub const SITE_FAMILY_FOLD_BINDING: u32 = 9;

/// Stable family identifier for the successor witness binding.
pub const SITE_FAMILY_NEXT_WITNESS: u32 = 10;

/// Stable family identifier for root public commitments and opening points.
pub const SITE_FAMILY_ROOT_STATEMENT: u32 = 11;

/// Stable family identifier for terminal response messages.
pub const SITE_FAMILY_TERMINAL: u32 = 12;

/// Proof-stream operation kind recorded by diagnostic context metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtocolMessageKind {
    /// A public or derived value supplied outside the argument string.
    PublicValue = 1,
    /// A bounded variable payload's length atom.
    ProofLength = 2,
    /// One or more canonical proof atoms.
    ProofAtoms = 3,
    /// A verifier challenge group.
    Challenge = 4,
    /// A proof-of-work nonce atom.
    GrindingNonce = 5,
    /// A proof-of-work predicate challenge.
    GrindingPredicate = 6,
    /// A fold-response search nonce atom.
    FoldResponseNonce = 7,
}

/// Canonical 32-byte identity of one protocol site.
///
/// Unused coordinates are zero. Callers derive every coordinate from validated
/// public schedule state; proof input never selects a site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProtocolSiteId {
    /// Protocol family, such as sumcheck or folding.
    pub family: u32,
    /// Invocation within the enclosing public schedule.
    pub invocation: u32,
    /// Recursive fold level.
    pub level: u32,
    /// Stage within a level.
    pub stage: u32,
    /// Round within a stage.
    pub round: u32,
    /// Commitment group within a round.
    pub group: u32,
    /// Base-field limb within an extension challenge.
    pub limb: u32,
    /// Family-specific public discriminator, such as grinding widths.
    pub detail: u32,
}

impl ProtocolSiteId {
    /// Encode this site identity in its fixed canonical layout.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 32] {
        let mut encoded = [0u8; 32];
        let family = self.family.to_le_bytes();
        let invocation = self.invocation.to_le_bytes();
        let level = self.level.to_le_bytes();
        let stage = self.stage.to_le_bytes();
        let round = self.round.to_le_bytes();
        let group = self.group.to_le_bytes();
        let limb = self.limb.to_le_bytes();
        let detail = self.detail.to_le_bytes();
        encoded[0] = family[0];
        encoded[1] = family[1];
        encoded[2] = family[2];
        encoded[3] = family[3];
        encoded[4] = invocation[0];
        encoded[5] = invocation[1];
        encoded[6] = invocation[2];
        encoded[7] = invocation[3];
        encoded[8] = level[0];
        encoded[9] = level[1];
        encoded[10] = level[2];
        encoded[11] = level[3];
        encoded[12] = stage[0];
        encoded[13] = stage[1];
        encoded[14] = stage[2];
        encoded[15] = stage[3];
        encoded[16] = round[0];
        encoded[17] = round[1];
        encoded[18] = round[2];
        encoded[19] = round[3];
        encoded[20] = group[0];
        encoded[21] = group[1];
        encoded[22] = group[2];
        encoded[23] = group[3];
        encoded[24] = limb[0];
        encoded[25] = limb[1];
        encoded[26] = limb[2];
        encoded[27] = limb[3];
        encoded[28] = detail[0];
        encoded[29] = detail[1];
        encoded[30] = detail[2];
        encoded[31] = detail[3];
        encoded
    }
}

/// Spongefish prover state used by Akita.
pub type ProverChannel = ProverState<TranscriptSponge>;

#[derive(Clone, Copy)]
struct FramedBytes<'a> {
    bytes: &'a [u8],
    len: u64,
}

impl<'a> FramedBytes<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, AkitaError> {
        Ok(Self {
            bytes,
            len: u64::try_from(bytes.len()).map_err(|_| {
                AkitaError::InvalidInput("framed transcript input length does not fit u64".into())
            })?,
        })
    }
}

impl Encoding<[u8]> for FramedBytes<'_> {
    fn encode(&self) -> impl AsRef<[u8]> {
        let mut out = Vec::with_capacity(8 + self.bytes.len());
        out.extend_from_slice(&self.len.to_le_bytes());
        out.extend_from_slice(self.bytes);
        out
    }
}

fn proof_stream_protocol_id(version: ProofStreamVersion) -> [u8; 64] {
    #[cfg(feature = "transcript-blake2b")]
    let name = "blake2b";
    #[cfg(feature = "transcript-keccak")]
    let name = "keccak";
    let ver = version.segment();
    protocol_id(format_args!("akita-pcs/native-proof-stream/{ver}/{name}"))
}

fn domain<'a>(
    version: ProofStreamVersion,
    session: &'a [u8],
    instance: &'a [u8],
) -> Result<
    DomainSeparator<
        spongefish::WithInstance<FramedBytes<'a>>,
        spongefish::WithSession<FramedBytes<'a>>,
    >,
    AkitaError,
> {
    Ok(
        DomainSeparator::<WithoutInstance>::new(proof_stream_protocol_id(version))
            .session(FramedBytes::new(session)?)
            .instance(FramedBytes::new(instance)?),
    )
}

/// Construct a bound prover channel under the base proof-stream version.
///
/// # Errors
///
/// Returns [`AkitaError::InvalidInput`] when a framed transcript input length exceeds
/// `u64`.
pub fn new_prover_channel(session: &[u8], instance: &[u8]) -> Result<ProverChannel, AkitaError> {
    new_prover_channel_for(ProofStreamVersion::V7, session, instance)
}

/// Extension seam: construct a bound prover channel under `version`, which a
/// configuration selects at runtime (terminal quotient certificate).
///
/// # Errors
///
/// As [`new_prover_channel`].
pub fn new_prover_channel_for(
    version: ProofStreamVersion,
    session: &[u8],
    instance: &[u8],
) -> Result<ProverChannel, AkitaError> {
    Ok(domain(version, session, instance)?.to_prover(TranscriptSponge::default()))
}

/// Construct a bound verifier channel over one proof byte string under the
/// base proof-stream version.
///
/// # Errors
///
/// Returns [`AkitaError::InvalidInput`] when a framed transcript input length exceeds
/// `u64`.
pub fn new_verifier_channel<'proof>(
    session: &[u8],
    instance: &[u8],
    proof: &'proof [u8],
) -> Result<VerifierChannel<'proof>, AkitaError> {
    new_verifier_channel_for(ProofStreamVersion::V7, session, instance, proof)
}

/// Extension seam: construct a bound verifier channel under `version`, which
/// a configuration selects at runtime (terminal quotient certificate).
///
/// # Errors
///
/// As [`new_verifier_channel`].
pub fn new_verifier_channel_for<'proof>(
    version: ProofStreamVersion,
    session: &[u8],
    instance: &[u8],
    proof: &'proof [u8],
) -> Result<VerifierChannel<'proof>, AkitaError> {
    Ok(VerifierChannel::new(
        domain(version, session, instance)?.to_verifier(TranscriptSponge::default(), proof),
    ))
}

/// A fixed-width, canonical field atom for proof transport.
///
/// This is deliberately an atom rather than a shape-aware container. Runtime
/// proof shapes are enforced by schedule-derived receive loops in the protocol
/// crates, while this decoder only accepts canonical field representatives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldAtom<F: CanonicalEncoding>(F);

impl<F: CanonicalEncoding> FieldAtom<F> {
    /// Wrap one field element for proof transport.
    #[must_use]
    pub const fn new(value: F) -> Self {
        Self(value)
    }

    /// Return the wrapped field element.
    #[must_use]
    pub const fn into_inner(self) -> F {
        self.0
    }
}

/// Canonical extension-field proof atom with transactional decoding.
///
/// All base coordinates are decoded against a local cursor. The caller's
/// cursor advances only after every coordinate is present and canonical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtensionAtom<F, E> {
    value: E,
    _base: PhantomData<F>,
}

impl<F, E> ExtensionAtom<F, E> {
    /// Wrap one extension element for proof transport.
    #[must_use]
    pub const fn new(value: E) -> Self {
        Self {
            value,
            _base: PhantomData,
        }
    }

    /// Return the wrapped extension element.
    #[must_use]
    pub fn into_inner(self) -> E {
        self.value
    }
}

impl<F, E> Encoding<[u8]> for ExtensionAtom<F, E>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
{
    fn encode(&self) -> impl AsRef<[u8]> {
        let coefficients = self.value.to_base_vec();
        AtomBytes::with_len(coefficients.len() * F::NUM_BYTES, |out| {
            for (coefficient, chunk) in coefficients.iter().zip(out.chunks_exact_mut(F::NUM_BYTES))
            {
                coefficient.to_bytes_le(chunk);
            }
        })
    }
}

impl<F, E> NargDeserialize for ExtensionAtom<F, E>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
{
    fn deserialize_from_narg(buf: &mut &[u8]) -> Result<Self, VerificationError> {
        let mut remaining = *buf;
        let mut coefficients = Vec::new();
        coefficients
            .try_reserve_exact(E::DEGREE)
            .map_err(|_| VerificationError)?;
        for _ in 0..E::DEGREE {
            coefficients.push(FieldAtom::<F>::deserialize_from_narg(&mut remaining)?.into_inner());
        }
        let value = E::from_base_slice(&coefficients);
        *buf = remaining;
        Ok(Self::new(value))
    }
}

impl<F: CanonicalEncoding> Encoding<[u8]> for FieldAtom<F> {
    fn encode(&self) -> impl AsRef<[u8]> {
        AtomBytes::with_len(F::NUM_BYTES, |out| self.0.to_bytes_le(out))
    }
}

/// Encoded bytes of one proof atom, inline when they fit in 64 bytes.
///
/// Spongefish encodes every received and public atom before absorbing it;
/// writing the encoding into an inline, word-aligned buffer avoids
/// allocating it on the heap.
enum AtomBytes {
    Inline { bytes: InlineAtomBytes, len: usize },
    Heap(Vec<u8>),
}

#[repr(C, align(8))]
struct InlineAtomBytes([u8; 64]);

impl AtomBytes {
    fn with_len(len: usize, fill: impl FnOnce(&mut [u8])) -> Self {
        let mut bytes = InlineAtomBytes([0; 64]);
        if let Some(out) = bytes.0.get_mut(..len) {
            fill(out);
            Self::Inline { bytes, len }
        } else {
            let mut out = vec![0; len];
            fill(&mut out);
            Self::Heap(out)
        }
    }
}

impl AsRef<[u8]> for AtomBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            // `with_len` builds `Inline` only when `len` fits the buffer.
            Self::Inline { bytes, len } => bytes.0.get(..*len).unwrap_or_default(),
            Self::Heap(bytes) => bytes,
        }
    }
}

impl<F: CanonicalEncoding> NargDeserialize for FieldAtom<F> {
    fn deserialize_from_narg(buf: &mut &[u8]) -> Result<Self, VerificationError> {
        let (encoded, remaining) = buf
            .split_at_checked(F::NUM_BYTES)
            .ok_or(VerificationError)?;
        let value = F::from_bytes_le_checked(encoded).ok_or(VerificationError)?;
        *buf = remaining;
        Ok(Self(value))
    }
}

/// Fixed-width little-endian `u128` proof atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct U128Atom(u128);

impl U128Atom {
    /// Wrap an integer for proof transport.
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the wrapped integer.
    #[must_use]
    pub const fn into_inner(self) -> u128 {
        self.0
    }
}

impl Encoding<[u8]> for U128Atom {
    fn encode(&self) -> impl AsRef<[u8]> {
        self.0.to_le_bytes()
    }
}

impl NargDeserialize for U128Atom {
    fn deserialize_from_narg(buf: &mut &[u8]) -> Result<Self, VerificationError> {
        let (encoded, remaining) = buf.split_at_checked(16).ok_or(VerificationError)?;
        let bytes: [u8; 16] = encoded.try_into().map_err(|_| VerificationError)?;
        *buf = remaining;
        Ok(Self(u128::from_le_bytes(bytes)))
    }
}

/// Emit one canonical field atom and absorb the same bytes.
pub fn send_field<F: CanonicalEncoding>(state: &mut ProverChannel, value: F) {
    state.prover_message(&FieldAtom::new(value));
}

/// Receive one canonical field atom, rejecting noncanonical representatives.
pub fn receive_field<F: CanonicalEncoding>(
    state: &mut VerifierChannel<'_>,
) -> Result<F, AkitaError> {
    state
        .prover_message::<FieldAtom<F>>()
        .map(FieldAtom::into_inner)
}

/// Emit one extension-field proof atom as ordered canonical base coordinates.
pub fn send_extension<F, E>(state: &mut ProverChannel, value: E)
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
{
    state.prover_message(&ExtensionAtom::<F, E>::new(value));
}

/// Receive one extension-field proof atom from canonical base coordinates.
pub fn receive_extension<F, E>(state: &mut VerifierChannel<'_>) -> Result<E, AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
{
    state
        .prover_message::<ExtensionAtom<F, E>>()
        .map(ExtensionAtom::into_inner)
}

const BYTE_CHUNK_BYTES: usize = 1024;
const BYTE_TAIL_CHUNK_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ByteChunk<const N: usize>([u8; N]);

impl<const N: usize> Encoding<[u8]> for ByteChunk<N> {
    fn encode(&self) -> impl AsRef<[u8]> {
        self.0.as_slice()
    }
}

impl<const N: usize> NargDeserialize for ByteChunk<N> {
    fn deserialize_from_narg(buf: &mut &[u8]) -> Result<Self, VerificationError> {
        let (encoded, remaining) = buf.split_at_checked(N).ok_or(VerificationError)?;
        let bytes = encoded.try_into().map_err(|_| VerificationError)?;
        *buf = remaining;
        Ok(Self(bytes))
    }
}

fn send_byte_chunks<'a, const N: usize>(
    state: &mut ProverChannel,
    mut bytes: &'a [u8],
) -> &'a [u8] {
    while bytes.len() >= N {
        let (chunk, remaining) = bytes.split_at(N);
        let chunk: ByteChunk<N> = ByteChunk(chunk.try_into().expect("chunk length is fixed"));
        state.prover_message(&chunk);
        bytes = remaining;
    }
    bytes
}

fn receive_byte_chunks<const N: usize>(
    state: &mut VerifierChannel<'_>,
    len: usize,
    bytes: &mut Vec<u8>,
) -> Result<usize, AkitaError> {
    for _ in 0..len / N {
        let chunk = state.prover_message::<ByteChunk<N>>()?;
        bytes.extend_from_slice(&chunk.0);
    }
    Ok(len % N)
}

/// Emit a schedule-bounded byte sequence in fixed-size chunks.
///
/// Spongefish absorption is associative, so this emits and absorbs exactly the
/// same byte string as one-byte messages while avoiding one call per byte.
pub fn send_bytes(state: &mut ProverChannel, bytes: &[u8]) {
    let bytes = send_byte_chunks::<BYTE_CHUNK_BYTES>(state, bytes);
    let bytes = send_byte_chunks::<BYTE_TAIL_CHUNK_BYTES>(state, bytes);
    for &byte in bytes {
        state.prover_message(&[byte]);
    }
}

/// Receive an exact schedule-bounded number of bytes in fixed-size chunks.
pub fn receive_bytes(state: &mut VerifierChannel<'_>, len: usize) -> Result<Vec<u8>, AkitaError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|_| AkitaError::InvalidProof)?;
    let remaining = receive_byte_chunks::<BYTE_CHUNK_BYTES>(state, len, &mut bytes)?;
    let remaining = receive_byte_chunks::<BYTE_TAIL_CHUNK_BYTES>(state, remaining, &mut bytes)?;
    for _ in 0..remaining {
        bytes.push(state.prover_message::<[u8; 1]>()?[0]);
    }
    Ok(bytes)
}

/// Emit a schedule-bounded byte sequence as one diagnostically labelled proof group.
pub fn send_byte_group(
    state: &mut ProverChannel,
    site: ProtocolSiteId,
    bytes: &[u8],
) -> Result<(), AkitaError> {
    let len = u64::try_from(bytes.len())
        .map_err(|_| AkitaError::InvalidInput("proof byte group length does not fit u64".into()))?;
    prover_context(
        state,
        ProtocolContextRecord::new(
            site.to_bytes(),
            ProtocolMessageKind::ProofAtoms as u32,
            len,
            len,
            0,
        ),
    );
    send_bytes(state, bytes);
    Ok(())
}

/// Receive an exact schedule-bounded diagnostically labelled proof group.
pub fn receive_byte_group(
    state: &mut VerifierChannel<'_>,
    site: ProtocolSiteId,
    len: usize,
) -> Result<Vec<u8>, AkitaError> {
    let len_u64 = u64::try_from(len).map_err(|_| AkitaError::InvalidProof)?;
    verifier_context(
        state,
        ProtocolContextRecord::new(
            site.to_bytes(),
            ProtocolMessageKind::ProofAtoms as u32,
            len_u64,
            len_u64,
            0,
        ),
    );
    receive_bytes(state, len)
}

/// Length-atom record and payload site of one bounded byte payload at `site`.
fn bounded_bytes_sites(site: ProtocolSiteId) -> (ProtocolContextRecord, ProtocolSiteId) {
    let length_site = ProtocolSiteId { stage: 0, ..site };
    let length_record = ProtocolContextRecord::new(
        length_site.to_bytes(),
        ProtocolMessageKind::ProofLength as u32,
        1,
        4,
        0,
    );
    (length_record, ProtocolSiteId { stage: 1, ..site })
}

/// Emit a bounded variable byte payload with a `u32` length atom.
pub fn send_bounded_bytes(
    state: &mut ProverChannel,
    site: ProtocolSiteId,
    bytes: &[u8],
    max_len: usize,
) -> Result<(), AkitaError> {
    if bytes.len() > max_len {
        return Err(AkitaError::InvalidInput(
            "bounded proof payload length exceeds its scheduled maximum".into(),
        ));
    }
    let len = u32::try_from(bytes.len()).map_err(|_| {
        AkitaError::InvalidInput("bounded proof payload length does not fit u32".into())
    })?;
    let (length_record, payload_site) = bounded_bytes_sites(site);
    prover_context(state, length_record);
    state.prover_message(&len);
    send_byte_group(state, payload_site, bytes)
}

/// Receive a bounded variable byte payload after validating its length
/// atom and before allocating its body.
pub fn receive_bounded_bytes(
    state: &mut VerifierChannel<'_>,
    site: ProtocolSiteId,
    max_len: usize,
) -> Result<Vec<u8>, AkitaError> {
    let (length_record, payload_site) = bounded_bytes_sites(site);
    verifier_context(state, length_record);
    let len = state.prover_message::<u32>()?;
    let len = usize::try_from(len).map_err(|_| {
        state.invalidate();
        AkitaError::InvalidProof
    })?;
    if len > max_len {
        state.invalidate();
        return Err(AkitaError::InvalidProof);
    }
    receive_byte_group(state, payload_site, len)
}

fn public_bytes_record(
    site: ProtocolSiteId,
    len: usize,
) -> Result<ProtocolContextRecord, AkitaError> {
    let len = usize_to_u64(len, "public byte record length")?;
    Ok(ProtocolContextRecord::new(
        site.to_bytes(),
        ProtocolMessageKind::PublicValue as u32,
        len,
        len,
        0,
    ))
}

/// Absorb one public byte string and record its diagnostic site.
pub fn public_bytes<S: ProofChannel>(
    state: &mut S,
    site: ProtocolSiteId,
    bytes: &[u8],
) -> Result<(), AkitaError> {
    state.context(public_bytes_record(site, bytes.len())?);
    state.public(bytes);
    Ok(())
}

/// Draw a context-bound extension-field challenge.
pub fn ext_challenge<F, E, S>(state: &mut S, site: ProtocolSiteId) -> Result<E, AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
    S: ProofChannel,
{
    let mut coefficients = Vec::new();
    coefficients
        .try_reserve_exact(E::DEGREE)
        .map_err(|_| AkitaError::InvalidProof)?;
    for limb in 0..E::DEGREE {
        let mut limb_site = site;
        limb_site.limb = usize_to_u32(limb, "extension challenge limb index")?;
        state.context(ProtocolContextRecord::new(
            limb_site.to_bytes(),
            ProtocolMessageKind::Challenge as u32,
            0,
            0,
            field_challenge_bytes::<F>(),
        ));
        coefficients.push(state.field_challenge()?);
    }
    Ok(E::from_base_slice(&coefficients))
}

/// Fixed-width diagnostic record identifying logical message and challenge groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolContextRecord {
    /// Fixed domain for all Akita context records.
    pub domain: [u8; 32],
    /// Context-record format version.
    pub version: u32,
    /// Canonical protocol-site identity.
    pub site_id: [u8; 32],
    /// Message or challenge kind.
    pub kind: u32,
    /// Number of fixed-width atoms in the group.
    pub atom_count: u64,
    /// Total canonical encoded bytes in the group.
    pub encoded_bytes: u64,
    /// Number of challenge bytes squeezed after the record.
    pub challenge_bytes: u64,
}

impl ProtocolContextRecord {
    /// Construct a versioned context record.
    #[must_use]
    pub const fn new(
        site_id: [u8; 32],
        kind: u32,
        atom_count: u64,
        encoded_bytes: u64,
        challenge_bytes: u64,
    ) -> Self {
        Self {
            domain: CONTEXT_DOMAIN,
            version: PROTOCOL_VERSION,
            site_id,
            kind,
            atom_count,
            encoded_bytes,
            challenge_bytes,
        }
    }
}

/// Record prover-side protocol metadata when transcript diagnostics are enabled.
#[inline(always)]
pub fn prover_context(state: &mut ProverChannel, record: ProtocolContextRecord) {
    #[cfg(feature = "logging-transcript")]
    {
        crate::logging::record_context(record);
        crate::logging::record_proof_boundary(record, state.narg_string().len());
    }
    #[cfg(not(feature = "logging-transcript"))]
    let _ = (state, record);
}

/// Close the final diagnostic proof range at the authoritative argument end.
#[cfg(feature = "logging-transcript")]
pub fn finish_proof_ranges(state: &ProverChannel) {
    crate::logging::finish_proof_ranges(state.narg_string().len());
}

/// Record verifier-side protocol metadata when transcript diagnostics are enabled.
#[inline(always)]
pub fn verifier_context(_state: &mut VerifierChannel<'_>, record: ProtocolContextRecord) {
    #[cfg(feature = "logging-transcript")]
    crate::logging::record_context(record);
    #[cfg(not(feature = "logging-transcript"))]
    let _ = record;
}

/// A prover-side clone of only the public duplex state used to preview a
/// fold-response candidate.
///
/// The type deliberately exposes only the fold transition needed by Akita. It
/// cannot mutate the live argument string, the prover's private RNG, or a
/// grinding-plan cursor.
pub struct FoldPreview {
    sponge: TranscriptSponge,
}

impl FoldPreview {
    /// Clone the public state and absorb one candidate nonce message.
    #[must_use]
    pub fn new(state: &ProverChannel, nonce: u32) -> Self {
        let mut sponge = state.duplex_sponge_state.clone();
        sponge.absorb(NonceAtom::new(nonce).encode().as_ref());
        Self { sponge }
    }

    /// Absorb one public fold payload and squeeze its root.
    #[must_use]
    pub fn fold_root(&mut self, payload: &[u8]) -> [u8; crate::FOLD_CHALLENGE_SEED_LEN] {
        self.sponge.absorb(payload);
        let mut root = [0u8; crate::FOLD_CHALLENGE_SEED_LEN];
        self.sponge.squeeze(&mut root);
        root
    }
}

/// Absorb one public fold payload and draw its root live on the
/// prover side.
#[must_use]
pub fn prover_fold_root(
    state: &mut ProverChannel,
    record: ProtocolContextRecord,
    payload: &[u8],
) -> [u8; crate::FOLD_CHALLENGE_SEED_LEN] {
    prover_context(state, record);
    state.public_message(payload);
    state.verifier_message()
}

/// Absorb one public fold payload and draw its root live on the
/// verifier side.
pub fn verifier_fold_root(
    state: &mut VerifierChannel<'_>,
    record: ProtocolContextRecord,
    payload: &[u8],
) -> Result<[u8; crate::FOLD_CHALLENGE_SEED_LEN], AkitaError> {
    verifier_context(state, record);
    state.public_message(payload);
    state.verifier_message()
}

fn extension_group_record<F, E>(
    site: ProtocolSiteId,
    kind: ProtocolMessageKind,
    value_count: usize,
) -> Result<ProtocolContextRecord, AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
{
    let atom_count = value_count.checked_mul(E::DEGREE).ok_or_else(|| {
        AkitaError::InvalidInput("extension record atom count overflows usize".into())
    })?;
    let encoded_bytes = atom_count.checked_mul(F::NUM_BYTES).ok_or_else(|| {
        AkitaError::InvalidInput("extension record encoded byte count overflows usize".into())
    })?;
    Ok(ProtocolContextRecord::new(
        site.to_bytes(),
        kind as u32,
        usize_to_u64(atom_count, "extension record atom count")?,
        usize_to_u64(encoded_bytes, "extension record encoded byte count")?,
        0,
    ))
}

fn field_group_record<F>(
    site: ProtocolSiteId,
    kind: ProtocolMessageKind,
    value_count: usize,
) -> Result<ProtocolContextRecord, AkitaError>
where
    F: CanonicalEncoding,
{
    let encoded_bytes = value_count.checked_mul(F::NUM_BYTES).ok_or_else(|| {
        AkitaError::InvalidInput("field record encoded byte count overflows usize".into())
    })?;
    Ok(ProtocolContextRecord::new(
        site.to_bytes(),
        kind as u32,
        usize_to_u64(value_count, "field record value count")?,
        usize_to_u64(encoded_bytes, "field record encoded byte count")?,
        0,
    ))
}

/// Absorb a fixed-count group of public base-field values.
pub fn public_fields_prover<F>(
    state: &mut ProverChannel,
    site: ProtocolSiteId,
    values: &[F],
) -> Result<(), AkitaError>
where
    F: CanonicalEncoding,
{
    prover_context(
        state,
        field_group_record::<F>(site, ProtocolMessageKind::PublicValue, values.len())?,
    );
    for &value in values {
        state.public_message(&FieldAtom::new(value));
    }
    Ok(())
}

/// Absorb a fixed-count group of public base-field values.
pub fn public_fields_verifier<F>(
    state: &mut VerifierChannel<'_>,
    site: ProtocolSiteId,
    values: &[F],
) -> Result<(), AkitaError>
where
    F: CanonicalEncoding,
{
    verifier_context(
        state,
        field_group_record::<F>(site, ProtocolMessageKind::PublicValue, values.len())?,
    );
    for &value in values {
        state.public_message(&FieldAtom::new(value));
    }
    Ok(())
}

/// Emit a fixed-count group of canonical base-field proof atoms.
pub fn send_field_group<F>(
    state: &mut ProverChannel,
    site: ProtocolSiteId,
    values: &[F],
) -> Result<(), AkitaError>
where
    F: CanonicalEncoding,
{
    prover_context(
        state,
        field_group_record::<F>(site, ProtocolMessageKind::ProofAtoms, values.len())?,
    );
    for &value in values {
        send_field(state, value);
    }
    Ok(())
}

/// Receive a schedule-fixed group of canonical base-field proof atoms.
pub fn receive_field_group<F>(
    state: &mut VerifierChannel<'_>,
    site: ProtocolSiteId,
    value_count: usize,
) -> Result<Vec<F>, AkitaError>
where
    F: CanonicalEncoding,
{
    verifier_context(
        state,
        field_group_record::<F>(site, ProtocolMessageKind::ProofAtoms, value_count)?,
    );
    let mut values = Vec::new();
    values
        .try_reserve_exact(value_count)
        .map_err(|_| AkitaError::InvalidProof)?;
    for _ in 0..value_count {
        values.push(receive_field(state)?);
    }
    Ok(values)
}

/// Absorb a fixed-count group of public extension-field values.
pub fn public_extensions<F, E, S>(
    state: &mut S,
    site: ProtocolSiteId,
    values: &[E],
) -> Result<(), AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
    S: ProofChannel,
{
    state.context(extension_group_record::<F, E>(
        site,
        ProtocolMessageKind::PublicValue,
        values.len(),
    )?);
    for value in values {
        for coefficient in value.to_base_vec() {
            state.public(&FieldAtom::new(coefficient));
        }
    }
    Ok(())
}

/// Allocate `count` zeroed extension slots for an in-place exchange.
///
/// # Errors
///
/// Returns [`AkitaError::InvalidProof`] when the allocation cannot be reserved.
pub fn extension_slots<E: Field>(count: usize) -> Result<Vec<E>, AkitaError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| AkitaError::InvalidProof)?;
    values.resize(count, E::zero());
    Ok(values)
}

/// Exchange a schedule-fixed group of extension-field proof values in place.
///
/// The prover emits `values`; the verifier overwrites them with the received
/// atoms. The group length is public and comes from the schedule.
///
/// # Errors
///
/// Returns [`AkitaError::InvalidInput`] when the group record count overflows,
/// or [`AkitaError::InvalidProof`] when the verifier cannot decode an atom.
pub fn exchange_extension_group<F, E, S>(
    state: &mut S,
    site: ProtocolSiteId,
    values: &mut [E],
) -> Result<(), AkitaError>
where
    F: Field + CanonicalEncoding,
    E: ExtField<F>,
    S: ProofChannel,
{
    state.context(extension_group_record::<F, E>(
        site,
        ProtocolMessageKind::ProofAtoms,
        values.len(),
    )?);
    for value in values {
        let mut atom = ExtensionAtom::<F, E>::new(*value);
        state.exchange(&mut atom)?;
        *value = atom.into_inner();
    }
    Ok(())
}

/// Preview the predicate produced by a candidate grinding nonce.
///
/// Only the public duplex state is cloned. The live state, private prover RNG,
/// and argument string are not mutated.
#[must_use]
pub fn preview_grinding_predicate(
    state: &ProverChannel,
    nonce: u32,
) -> [u8; crate::GRINDING_PREDICATE_LEN] {
    let mut sponge = state.duplex_sponge_state.clone();
    sponge.absorb(NonceAtom::new(nonce).encode().as_ref());
    let mut predicate = [0u8; crate::GRINDING_PREDICATE_LEN];
    sponge.squeeze(&mut predicate);
    predicate
}

/// Search the canonical bounded nonce range against Spongefish previews.
///
/// A zero-bit target is a no-op and does not absorb either record.
#[must_use]
pub fn search_grinding_nonce(
    state: &ProverChannel,
    grind_bits: u8,
    nonce_bits: u8,
) -> Option<(u32, [u8; crate::GRINDING_PREDICATE_LEN])> {
    crate::grinding::search_grinding_nonce_with(grind_bits, nonce_bits, |nonce| {
        Some(preview_grinding_predicate(state, nonce))
    })
}

/// Commit a winning grinding nonce and draw its predicate.
#[must_use]
pub fn commit_grinding_nonce(
    state: &mut ProverChannel,
    nonce_record: ProtocolContextRecord,
    nonce: u32,
    predicate_record: ProtocolContextRecord,
) -> [u8; crate::GRINDING_PREDICATE_LEN] {
    prover_context(state, nonce_record);
    state.prover_message(&NonceAtom::new(nonce));
    prover_context(state, predicate_record);
    state.verifier_message()
}

/// Receive a grinding nonce and draw its predicate.
pub fn receive_grinding_nonce(
    state: &mut VerifierChannel<'_>,
    nonce_record: ProtocolContextRecord,
    predicate_record: ProtocolContextRecord,
) -> Result<(u32, [u8; crate::GRINDING_PREDICATE_LEN]), AkitaError> {
    verifier_context(state, nonce_record);
    let nonce = state.prover_message::<NonceAtom>()?.into_inner();
    verifier_context(state, predicate_record);
    Ok((nonce, state.verifier_message()?))
}

#[cfg(test)]
mod tests;
