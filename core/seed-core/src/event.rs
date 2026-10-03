use crate::{
    crypto::{hash32, verify_strict, CryptoError, Signature},
    id::{DeviceId, EventId, IdentityId, SpaceId},
    identity::{DeviceAuthorization, DeviceIdentity, IdentityDocument, IdentityError},
};

const EVENT_SIGNATURE_DOMAIN: &[u8] = b"seed:event-signature:v1\0";
const EVENT_ID_DOMAIN: &[u8] = b"seed:event-id:v1\0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventHeader {
    pub protocol_version: u16,
    pub space: SpaceId,
    pub author: IdentityId,
    pub device: DeviceId,
    pub sequence: u64,
    pub timestamp_ms: i64,
    pub schema: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    id: EventId,
    header: EventHeader,
    payload: Vec<u8>,
    signature: Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    FieldTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventBuildError {
    Encode(EncodeError),
    Identity(IdentityError),
    ActorMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventVerifyError {
    Encode(EncodeError),
    Identity(IdentityError),
    ActorMismatch,
    IdMismatch,
    Crypto(CryptoError),
}

impl Event {
    pub fn sign(
        identity: &IdentityDocument,
        authorization: &DeviceAuthorization,
        device: &DeviceIdentity,
        header: EventHeader,
        payload: Vec<u8>,
    ) -> Result<Self, EventBuildError> {
        authorization
            .verify_against(identity)
            .map_err(EventBuildError::Identity)?;

        if header.author != identity.id()
            || header.device != authorization.device_id()
            || header.device != device.id()
        {
            return Err(EventBuildError::ActorMismatch);
        }

        let unsigned = canonical_unsigned_bytes(&header, &payload)
            .map_err(EventBuildError::Encode)?;
        let signature_input = domain_wrap(EVENT_SIGNATURE_DOMAIN, &unsigned);
        let signature = device.sign(&signature_input);
        let id = derive_event_id(&unsigned, &signature);

        Ok(Self {
            id,
            header,
            payload,
            signature,
        })
    }

    pub const fn id(&self) -> EventId {
        self.id
    }

    pub fn header(&self) -> &EventHeader {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Deterministic internal framing for signatures/tests during Phase 1.
    ///
    /// This is not yet the frozen Seed wire format.
    pub fn canonical_unsigned_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        canonical_unsigned_bytes(&self.header, &self.payload)
    }

    pub fn verify(
        &self,
        identity: &IdentityDocument,
        authorization: &DeviceAuthorization,
    ) -> Result<(), EventVerifyError> {
        authorization
            .verify_against(identity)
            .map_err(EventVerifyError::Identity)?;

        if self.header.author != identity.id()
            || self.header.device != authorization.device_id()
        {
            return Err(EventVerifyError::ActorMismatch);
        }

        let unsigned = self
            .canonical_unsigned_bytes()
            .map_err(EventVerifyError::Encode)?;

        if derive_event_id(&unsigned, &self.signature) != self.id {
            return Err(EventVerifyError::IdMismatch);
        }

        let signature_input = domain_wrap(EVENT_SIGNATURE_DOMAIN, &unsigned);
        verify_strict(
            &authorization.device_public_key(),
            &signature_input,
            &self.signature,
        )
        .map_err(EventVerifyError::Crypto)
    }
}

fn canonical_unsigned_bytes(
    header: &EventHeader,
    payload: &[u8],
) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::with_capacity(160 + header.schema.len() + payload.len());
    out.extend_from_slice(&header.protocol_version.to_be_bytes());
    out.extend_from_slice(header.space.as_bytes());
    out.extend_from_slice(header.author.as_bytes());
    out.extend_from_slice(header.device.as_bytes());
    out.extend_from_slice(&header.sequence.to_be_bytes());
    out.extend_from_slice(&header.timestamp_ms.to_be_bytes());
    push_len_prefixed(&mut out, header.schema.as_bytes())?;
    push_len_prefixed(&mut out, payload)?;
    Ok(out)
}

fn derive_event_id(unsigned: &[u8], signature: &Signature) -> EventId {
    EventId::from_bytes(hash32(
        EVENT_ID_DOMAIN,
        &[unsigned, signature.as_bytes()],
    ))
}

fn domain_wrap(domain: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + bytes.len());
    out.extend_from_slice(domain);
    out.extend_from_slice(bytes);
    out
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), EncodeError> {
    let len = u32::try_from(bytes.len()).map_err(|_| EncodeError::FieldTooLarge)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    fn fixture() -> (
        RootIdentity,
        DeviceIdentity,
        DeviceAuthorization,
        EventHeader,
    ) {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 123_000);
        let header = EventHeader {
            protocol_version: PROTOCOL_VERSION,
            space: SpaceId::from_bytes([2; 32]),
            author: root.document().id(),
            device: device.id(),
            sequence: 7,
            timestamp_ms: 123_456,
            schema: "seed.message.text/v1".to_owned(),
        };
        (root, device, authorization, header)
    }

    #[test]
    fn signed_event_round_trip_verifies() {
        let (root, device, authorization, header) = fixture();
        let event = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            b"hello".to_vec(),
        )
        .unwrap();

        event.verify(root.document(), &authorization).unwrap();
    }

    #[test]
    fn event_id_is_deterministic_for_same_signed_content() {
        let (root, device, authorization, header) = fixture();

        let a = Event::sign(
            root.document(),
            &authorization,
            &device,
            header.clone(),
            b"hello".to_vec(),
        )
        .unwrap();
        let b = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            b"hello".to_vec(),
        )
        .unwrap();

        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let (root, device, authorization, header) = fixture();
        let mut event = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            b"hello".to_vec(),
        )
        .unwrap();

        event.payload = b"tampered".to_vec();

        assert_eq!(
            event.verify(root.document(), &authorization),
            Err(EventVerifyError::IdMismatch)
        );
    }
}
