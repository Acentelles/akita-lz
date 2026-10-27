//! Selected-row verifier catalog views.
//!
//! A view carries the complete catalog's row identities but the parameters of
//! only the rows a verifier needs, in a postcard encoding a guest decodes
//! without JSON parsing. Loading it repeats the JSON catalog's semantic audit
//! for every carried row and reproduces the complete catalog's digest from the
//! carried identity list. The view does not authenticate that list: a consumer
//! that pins the view bytes or binds the catalog digest into its transcript
//! gets the same identity as the full catalog; Akita itself binds only the
//! selected row.

use super::{
    catalog_digest, policy_digest, AkitaError, CatalogCoverage, PlannerPolicy,
    ScheduleCatalogArtifactRowV1, SparseChallengeConfig, ValidatedScheduleCatalog,
    AKITA_INSTANCE_DESCRIPTOR_VERSION, MAX_TRUSTED_CATALOG_ROWS,
    MAX_TRUSTED_SCHEDULE_ARTIFACT_ROW_BYTES,
};
use akita_params::{OpeningScheduleSelection, ScheduleRowDigest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const VERIFIER_MAGIC: [u8; 8] = *b"AKSCVFY1";

/// Byte bound for a view: the full identity list plus one JSON row's budget
/// for the carried rows. It bounds decoding before any count check runs.
const MAX_VERIFIER_VIEW_BYTES: usize =
    MAX_TRUSTED_CATALOG_ROWS * 32 + MAX_TRUSTED_SCHEDULE_ARTIFACT_ROW_BYTES;

#[derive(Serialize, Deserialize)]
struct BinaryVerifierCatalog {
    magic: [u8; 8],
    protocol_epoch: u32,
    policy_digest: [u8; 32],
    family_name: String,
    /// The ordered row digests, concatenated. A byte string encodes as a
    /// length and the raw bytes, so a guest copies it instead of decoding
    /// every byte as its own element.
    #[serde(with = "serde_bytes")]
    row_digests: Vec<u8>,
    rows: Vec<ScheduleCatalogArtifactRowV1>,
}

impl ValidatedScheduleCatalog {
    /// Encode a verifier view retaining only the requested rows while
    /// preserving the complete catalog's transcript identity. Omitted rows
    /// remain opaque digest commitments. The view serves verification with an
    /// existing setup; it cannot size a new setup or be exported as a complete
    /// JSON catalog.
    ///
    /// # Errors
    ///
    /// Returns [`AkitaError::InvalidSetup`] for an empty, oversized, or
    /// unresolvable selection, or an oversized encoding.
    pub fn to_verifier_view(
        &self,
        selections: &[OpeningScheduleSelection],
    ) -> Result<Vec<u8>, AkitaError> {
        if selections.is_empty() || selections.len() > MAX_TRUSTED_CATALOG_ROWS {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog selection is outside row limits".into(),
            ));
        }
        let selected = selections
            .iter()
            .map(|selection| selection.row_digest)
            .collect::<BTreeSet<_>>();
        let rows = selected
            .into_iter()
            .map(|row_digest| {
                self.resolve_selection(OpeningScheduleSelection { row_digest })
                    .map(|row| ScheduleCatalogArtifactRowV1 {
                        schedule: row.schedule().clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let row_digests = match &self.coverage {
            CatalogCoverage::Complete => self
                .rows()
                .map(|row| *row.selection().row_digest.as_bytes())
                .collect::<Vec<_>>()
                .concat(),
            CatalogCoverage::Selected { row_digests } => row_digests.concat(),
        };
        let artifact = BinaryVerifierCatalog {
            magic: VERIFIER_MAGIC,
            protocol_epoch: AKITA_INSTANCE_DESCRIPTOR_VERSION,
            policy_digest: self.policy_digest,
            family_name: self.family_name.clone(),
            row_digests,
            rows,
        };
        let bytes = postcard::to_allocvec(&artifact).map_err(|error| {
            AkitaError::InvalidSetup(format!("cannot encode verifier catalog: {error}"))
        })?;
        if bytes.len() > MAX_VERIFIER_VIEW_BYTES {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog exceeds byte limit".into(),
            ));
        }
        Ok(bytes)
    }

    /// Load a verifier view supplied by the application's trusted setup path.
    ///
    /// Checks format, canonical encoding, configuration binding, every carried
    /// row's semantic invariants, and that each carried row is in the carried
    /// identity list. Omitted identities do not resolve to parameters. The
    /// identity list is as trusted as the bytes: the caller owns provenance.
    ///
    /// # Errors
    ///
    /// Returns [`AkitaError::InvalidSetup`] for a malformed view, a
    /// configuration mismatch, or a carried row outside the commitment.
    pub fn from_verifier_view(
        bytes: &[u8],
        expected_family_name: &str,
        policy: &PlannerPolicy,
        ring_challenge_config: impl Fn(usize) -> Result<SparseChallengeConfig, AkitaError>,
    ) -> Result<Self, AkitaError> {
        if bytes.len() > MAX_VERIFIER_VIEW_BYTES {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog exceeds byte limit".into(),
            ));
        }
        let (artifact, rest): (BinaryVerifierCatalog, &[u8]) = postcard::take_from_bytes(bytes)
            .map_err(|error| {
                AkitaError::InvalidSetup(format!("invalid verifier catalog: {error}"))
            })?;
        if !rest.is_empty()
            || artifact.magic != VERIFIER_MAGIC
            || artifact.protocol_epoch != AKITA_INSTANCE_DESCRIPTOR_VERSION
        {
            return Err(AkitaError::InvalidSetup(
                "unsupported verifier catalog format or trailing bytes".into(),
            ));
        }
        if artifact.family_name != expected_family_name
            || artifact.policy_digest != policy_digest(policy)
        {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog does not match runtime config".into(),
            ));
        }
        if !artifact.row_digests.len().is_multiple_of(32) {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog identities are not whole digests".into(),
            ));
        }
        let row_digests = artifact
            .row_digests
            .chunks_exact(32)
            // `chunks_exact(32)` yields 32-byte slices, so the conversion holds.
            .map(|digest| <[u8; 32]>::try_from(digest).unwrap_or_default())
            .collect::<Vec<_>>();
        if row_digests.is_empty()
            || row_digests.len() > MAX_TRUSTED_CATALOG_ROWS
            || artifact.rows.is_empty()
            || artifact.rows.len() > row_digests.len()
            || row_digests.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog identities are not a bounded canonical list".into(),
            ));
        }
        let rows = artifact
            .rows
            .into_iter()
            .map(ScheduleCatalogArtifactRowV1::into_profile_and_schedule)
            .collect::<Result<Vec<_>, _>>()?;
        // The JSON catalog's semantic audit owns the carried rows. No supplied
        // identity can replace the digest computed from a row's validated contents.
        let mut catalog = Self::try_new(artifact.family_name, rows, policy, ring_challenge_config)?;
        for row in catalog.rows() {
            if row_digests
                .binary_search(row.selection().row_digest.as_bytes())
                .is_err()
            {
                return Err(AkitaError::InvalidSetup(
                    "selected row is absent from the catalog commitment".into(),
                ));
            }
        }
        catalog.catalog_digest = catalog_digest(
            &catalog.family_name,
            catalog.policy_digest,
            row_digests
                .iter()
                .copied()
                .map(ScheduleRowDigest::from_bytes),
        );
        catalog.coverage = CatalogCoverage::Selected { row_digests };
        // Like the JSON artifact, a view has one encoding: re-encoding rejects
        // overlong varints and carried rows out of digest order.
        let selections = catalog
            .rows()
            .map(|row| row.selection())
            .collect::<Vec<_>>();
        if catalog.to_verifier_view(&selections)? != bytes {
            return Err(AkitaError::InvalidSetup(
                "verifier catalog is not canonically encoded".into(),
            ));
        }
        Ok(catalog)
    }
}

/// A selected-row verifier view with the values that
/// [`ValidatedScheduleCatalog::from_verifier_view`] derived from it when it
/// was loaded ahead of time (for example while building a guest image).
///
/// It is a cache, not a source of trust: [`ValidatedScheduleCatalog::from_prevalidated_verifier_view`]
/// uses it only for bytes equal to `view`, and the bytes keep the provenance
/// their caller gives them.
#[cfg(feature = "prevalidated-catalog")]
#[derive(Clone, Copy, Debug)]
pub struct PrevalidatedVerifierView<'a> {
    /// The exact view bytes that were validated.
    pub view: &'a [u8],
    /// The runtime policy digest the view was validated under.
    pub policy_digest: [u8; 32],
    /// The catalog digest reproduced from the view's identity list.
    pub catalog_digest: [u8; 32],
    /// The row digest of every carried row, in the view's row order.
    pub carried_row_digests: &'a [[u8; 32]],
}

/// What [`ValidatedScheduleCatalog::prevalidate_verifier_view`] records.
#[cfg(feature = "prevalidated-catalog")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrevalidatedVerifierRecord {
    /// The runtime policy digest.
    pub policy_digest: [u8; 32],
    /// The catalog digest.
    pub catalog_digest: [u8; 32],
    /// The carried rows' digests, in the view's row order.
    pub carried_row_digests: Vec<[u8; 32]>,
}

#[cfg(feature = "prevalidated-catalog")]
impl ValidatedScheduleCatalog {
    /// Load `bytes` through [`Self::from_verifier_view`] and record the values
    /// it derived. The record is accepted only if
    /// [`Self::from_prevalidated_verifier_view`] rebuilds a catalog equal to
    /// the loaded one (compared by its complete `Debug` form).
    ///
    /// # Errors
    ///
    /// Returns the loading error, or [`AkitaError::InvalidSetup`] when the
    /// rebuilt catalog differs.
    pub fn prevalidate_verifier_view(
        bytes: &[u8],
        expected_family_name: &str,
        policy: &PlannerPolicy,
        ring_challenge_config: impl Fn(usize) -> Result<SparseChallengeConfig, AkitaError>,
    ) -> Result<PrevalidatedVerifierRecord, AkitaError> {
        let catalog =
            Self::from_verifier_view(bytes, expected_family_name, policy, ring_challenge_config)?;
        let record = PrevalidatedVerifierRecord {
            policy_digest: catalog.policy_digest,
            catalog_digest: catalog.catalog_digest,
            carried_row_digests: catalog
                .rows()
                .map(|row| *row.selection().row_digest.as_bytes())
                .collect(),
        };
        let prevalidated = PrevalidatedVerifierView {
            view: bytes,
            policy_digest: record.policy_digest,
            catalog_digest: record.catalog_digest,
            carried_row_digests: &record.carried_row_digests,
        };
        let rebuilt = Self::from_prevalidated_verifier_view(
            bytes,
            &prevalidated,
            expected_family_name,
            policy,
        )
        .ok_or_else(|| {
            AkitaError::InvalidSetup("prevalidated verifier view was not used".into())
        })?;
        if format!("{rebuilt:?}") != format!("{catalog:?}") {
            return Err(AkitaError::InvalidSetup(
                "prevalidated verifier view rebuilds a different catalog".into(),
            ));
        }
        Ok(record)
    }

    /// Rebuild the catalog that [`Self::from_verifier_view`] returns for
    /// `bytes`, from a [`PrevalidatedVerifierView`] of the same bytes, without
    /// repeating the row audit, the catalog digest or the canonical re-encode.
    ///
    /// In this order it requires: `bytes` has `prevalidated.view`'s length,
    /// `bytes` equals `prevalidated.view` byte for byte, and the policy digest
    /// of `policy` equals `prevalidated.policy_digest`; then that the decoded
    /// view names `expected_family_name` and that policy digest, carries
    /// exactly the recorded number of rows, and that every carried row passes
    /// its topology check. It returns `None` on any miss, and the caller then
    /// loads the bytes with [`Self::from_verifier_view`].
    pub fn from_prevalidated_verifier_view(
        bytes: &[u8],
        prevalidated: &PrevalidatedVerifierView<'_>,
        expected_family_name: &str,
        policy: &PlannerPolicy,
    ) -> Option<Self> {
        if bytes.len() != prevalidated.view.len()
            || bytes != prevalidated.view
            || policy_digest(policy) != prevalidated.policy_digest
        {
            return None;
        }
        let (artifact, rest): (BinaryVerifierCatalog, &[u8]) =
            postcard::take_from_bytes(bytes).ok()?;
        if !rest.is_empty()
            || artifact.family_name != expected_family_name
            || artifact.policy_digest != prevalidated.policy_digest
            || artifact.rows.len() != prevalidated.carried_row_digests.len()
            || !artifact.row_digests.len().is_multiple_of(32)
        {
            return None;
        }
        let mut resolved = Vec::with_capacity(artifact.rows.len());
        for (row, digest) in artifact
            .rows
            .into_iter()
            .zip(prevalidated.carried_row_digests)
        {
            let (profiles, schedule) = row.into_profile_and_schedule().ok()?;
            resolved.push(crate::resolve::ResolvedScheduleRow::from_prevalidated(
                OpeningScheduleSelection {
                    row_digest: ScheduleRowDigest::from_bytes(*digest),
                },
                profiles,
                schedule,
            ));
        }
        resolved.sort_by_key(|row| row.selection().row_digest);
        let mut rows_by_key = resolved
            .iter()
            .enumerate()
            .map(|(index, row)| {
                (
                    super::key_for_profiles(row.profiles()).canonical_order_key(),
                    index,
                )
            })
            .collect::<Vec<_>>();
        rows_by_key.sort_by(|(left_key, left_index), (right_key, right_index)| {
            left_key.cmp(right_key).then_with(|| {
                resolved[*left_index]
                    .selection()
                    .row_digest
                    .cmp(&resolved[*right_index].selection().row_digest)
            })
        });
        let mut row_digests = vec![[0u8; 32]; artifact.row_digests.len() / 32];
        row_digests
            .as_flattened_mut()
            .copy_from_slice(&artifact.row_digests);
        Some(Self {
            family_name: artifact.family_name,
            policy_digest: prevalidated.policy_digest,
            catalog_digest: prevalidated.catalog_digest,
            coverage: CatalogCoverage::Selected { row_digests },
            rows_by_digest: resolved,
            rows_by_key,
        })
    }
}
