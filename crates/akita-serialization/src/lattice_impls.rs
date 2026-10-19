//! Akita's wire format for Jolt's public-matrix seed: the one-byte
//! derivation tag, then the 32 seed bytes (`PublicMatrixSeed`'s canonical
//! bytes). Setup digests hash exactly these bytes.

use std::io::{Read, Write};

use jolt_crypto::lattice::{PublicMatrixDerivation, PublicMatrixSeed};

use crate::{AkitaDeserialize, AkitaSerialize, Compress, SerializationError, Valid, Validate};

impl Valid for PublicMatrixSeed {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl AkitaSerialize for PublicMatrixSeed {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        writer.write_all(&self.to_canonical_bytes())?;
        Ok(())
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        Self::CANONICAL_LEN
    }
}

impl AkitaDeserialize for PublicMatrixSeed {
    type Context = ();

    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
        _ctx: &(),
    ) -> Result<Self, SerializationError> {
        let tag = u8::deserialize_with_mode(&mut reader, compress, validate, &())?;
        let derivation = PublicMatrixDerivation::from_tag(tag).ok_or_else(|| {
            SerializationError::InvalidData(format!(
                "unsupported public matrix derivation tag {tag}"
            ))
        })?;
        let mut seed = [0u8; 32];
        reader.read_exact(&mut seed)?;
        Ok(Self { derivation, seed })
    }
}
