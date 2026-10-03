use crate::{
    crypto::{CryptoError, Signature, Verifier},
    id::{DeviceId, EventId, IdentityId, SpaceId},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventHeader {
    pub protocol_version: u16,
    pub id: EventId,
    pub space: SpaceId,
    pub author: IdentityId,
    pub device: DeviceId,
    pub sequence: u64,
    pub timestamp_ms: i64,
    pub schema: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub header: EventHeader,
    pub payload: Vec<u8>,
    pub signature: Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    FieldTooLarge,
}

impl Event {
    /// Deterministic internal framing for signatures/tests during Phase 0.
    ///
    /// This is NOT yet the frozen Seed wire format. ADR-0003 still needs to
    /// compare CBOR/MessagePack/Protobuf/custom canonical encoding.
    pub fn canonical_unsigned_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let mut out = Vec::with_capacity(160 + self.header.schema.len() + self.payload.len());
        out.extend_from_slice(&self.header.protocol_version.to_be_bytes());
        out.extend_from_slice(self.header.id.as_bytes());
        out.extend_from_slice(self.header.space.as_bytes());
        out.extend_from_slice(self.header.author.as_bytes());
        out.extend_from_slice(self.header.device.as_bytes());
        out.extend_from_slice(&self.header.sequence.to_be_bytes());
        out.extend_from_slice(&self.header.timestamp_ms.to_be_bytes());
        push_len_prefixed(&mut out, self.header.schema.as_bytes())?;
        push_len_prefixed(&mut out, &self.payload)?;
        Ok(out)
    }

    pub fn verify_with(&self, verifier: &impl Verifier) -> Result<(), EventVerifyError> {
        let bytes = self
            .canonical_unsigned_bytes()
            .map_err(EventVerifyError::Encode)?;
        verifier
            .verify(&self.header.author, &bytes, &self.signature)
            .map_err(EventVerifyError::Crypto)
    }
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), EncodeError> {
    let len = u32::try_from(bytes.len()).map_err(|_| EncodeError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventVerifyError {
    Encode(EncodeError),
    Crypto(CryptoError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> Event {
        Event {
            header: EventHeader {
                protocol_version: 1,
                id: EventId::from_bytes([1; 32]),
                space: SpaceId::from_bytes([2; 32]),
                author: IdentityId::from_bytes([3; 32]),
                device: DeviceId::from_bytes([4; 32]),
                sequence: 7,
                timestamp_ms: 123_456,
                schema: "seed.message.text/v1".to_owned(),
            },
            payload: b"hello".to_vec(),
            signature: Signature::from_bytes(vec![9; 64]),
        }
    }

    #[test]
    fn unsigned_encoding_is_deterministic() {
        let a = sample_event().canonical_unsigned_bytes().unwrap();
        let b = sample_event().canonical_unsigned_bytes().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn signature_is_not_inside_signed_bytes() {
        let mut a = sample_event();
        let mut b = sample_event();
        a.signature = Signature::from_bytes(vec![1; 64]);
        b.signature = Signature::from_bytes(vec![2; 64]);
        assert_eq!(
            a.canonical_unsigned_bytes().unwrap(),
            b.canonical_unsigned_bytes().unwrap()
        );
    }
}
