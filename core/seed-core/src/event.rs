use crate::{
    codec::{DecodeError as CodecDecodeError, Decoder, EncodeError as CodecEncodeError, Encoder},
    crypto::{hash32, verify_strict, CryptoError, Signature},
    id::{DeviceId, EventId, IdentityId, SpaceId},
    identity::{DeviceAuthorization, DeviceIdentity, IdentityDocument, IdentityError},
    PROTOCOL_VERSION,
};

const EVENT_WIRE_MAGIC: &[u8; 4] = b"SEVT";
const EVENT_WIRE_VERSION: u16 = 1;
const EVENT_SIGNATURE_DOMAIN: &[u8] = b"seed:event-signature:v1\0";
const EVENT_ID_DOMAIN: &[u8] = b"seed:event-id:v1\0";

pub const MAX_EVENT_SCHEMA_BYTES: usize = 4 * 1024;
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_EVENT_BODY_BYTES: usize = MAX_EVENT_PAYLOAD_BYTES + MAX_EVENT_SCHEMA_BYTES + 256;
pub const MAX_EVENT_WIRE_BYTES: usize = 4 + 2 + 4 + MAX_EVENT_BODY_BYTES + 64;

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
    SchemaTooLarge,
    PayloadTooLarge,
    BodyTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDecodeError {
    UnexpectedEof,
    TrailingBytes,
    InvalidMagic,
    UnsupportedWireVersion,
    InvalidUtf8,
    SchemaTooLarge,
    PayloadTooLarge,
    BodyTooLarge,
    NonCanonicalEncoding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventBuildError {
    Encode(EncodeError),
    Identity(IdentityError),
    ActorMismatch,
    UnsupportedProtocolVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventVerifyError {
    Encode(EncodeError),
    Identity(IdentityError),
    ActorMismatch,
    IdMismatch,
    UnsupportedProtocolVersion,
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
        if header.protocol_version != PROTOCOL_VERSION {
            return Err(EventBuildError::UnsupportedProtocolVersion);
        }

        authorization
            .verify_against(identity)
            .map_err(EventBuildError::Identity)?;

        if header.author != identity.id()
            || header.device != authorization.device_id()
            || header.device != device.id()
        {
            return Err(EventBuildError::ActorMismatch);
        }

        let unsigned =
            canonical_unsigned_bytes(&header, &payload).map_err(EventBuildError::Encode)?;
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

    /// Canonical Event record used by the persistent-log spike.
    ///
    /// The wrapper is versioned independently from the signed unsigned body.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let unsigned = self.canonical_unsigned_bytes()?;
        let mut encoder = Encoder::with_capacity(4 + 2 + 4 + unsigned.len() + 64);
        encoder.fixed(EVENT_WIRE_MAGIC);
        encoder.u16(EVENT_WIRE_VERSION);
        encoder.bytes(&unsigned).map_err(map_encode_error)?;
        encoder.fixed(self.signature.as_bytes());
        let bytes = encoder.finish();

        if bytes.len() > MAX_EVENT_WIRE_BYTES {
            return Err(EncodeError::BodyTooLarge);
        }

        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EventDecodeError> {
        if bytes.len() > MAX_EVENT_WIRE_BYTES {
            return Err(EventDecodeError::BodyTooLarge);
        }

        let mut decoder = Decoder::new(bytes);
        let magic: [u8; 4] = decoder.fixed().map_err(map_decode_error)?;
        if &magic != EVENT_WIRE_MAGIC {
            return Err(EventDecodeError::InvalidMagic);
        }

        let wire_version = decoder.u16().map_err(map_decode_error)?;
        if wire_version != EVENT_WIRE_VERSION {
            return Err(EventDecodeError::UnsupportedWireVersion);
        }

        let unsigned = decoder.bytes().map_err(map_decode_error)?;
        if unsigned.len() > MAX_EVENT_BODY_BYTES {
            return Err(EventDecodeError::BodyTooLarge);
        }

        let signature = Signature::from_bytes(decoder.fixed().map_err(map_decode_error)?);
        decoder.finish().map_err(map_decode_error)?;

        let (header, payload) = decode_unsigned(unsigned)?;
        let event = Self {
            id: derive_event_id(unsigned, &signature),
            header,
            payload,
            signature,
        };

        let reencoded = event
            .canonical_bytes()
            .map_err(|_| EventDecodeError::NonCanonicalEncoding)?;
        if reencoded != bytes {
            return Err(EventDecodeError::NonCanonicalEncoding);
        }

        Ok(event)
    }

    pub fn verify(
        &self,
        identity: &IdentityDocument,
        authorization: &DeviceAuthorization,
    ) -> Result<(), EventVerifyError> {
        if self.header.protocol_version != PROTOCOL_VERSION {
            return Err(EventVerifyError::UnsupportedProtocolVersion);
        }

        authorization
            .verify_against(identity)
            .map_err(EventVerifyError::Identity)?;

        if self.header.author != identity.id() || self.header.device != authorization.device_id() {
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

fn canonical_unsigned_bytes(header: &EventHeader, payload: &[u8]) -> Result<Vec<u8>, EncodeError> {
    if header.schema.len() > MAX_EVENT_SCHEMA_BYTES {
        return Err(EncodeError::SchemaTooLarge);
    }
    if payload.len() > MAX_EVENT_PAYLOAD_BYTES {
        return Err(EncodeError::PayloadTooLarge);
    }

    let mut encoder = Encoder::with_capacity(160 + header.schema.len() + payload.len());
    encoder.u16(header.protocol_version);
    encoder.fixed(header.space.as_bytes());
    encoder.fixed(header.author.as_bytes());
    encoder.fixed(header.device.as_bytes());
    encoder.u64(header.sequence);
    encoder.i64(header.timestamp_ms);
    encoder
        .bytes(header.schema.as_bytes())
        .map_err(map_encode_error)?;
    encoder.bytes(payload).map_err(map_encode_error)?;
    let bytes = encoder.finish();

    if bytes.len() > MAX_EVENT_BODY_BYTES {
        return Err(EncodeError::BodyTooLarge);
    }

    Ok(bytes)
}

fn decode_unsigned(bytes: &[u8]) -> Result<(EventHeader, Vec<u8>), EventDecodeError> {
    let mut decoder = Decoder::new(bytes);
    let protocol_version = decoder.u16().map_err(map_decode_error)?;
    if protocol_version != PROTOCOL_VERSION {
        return Err(EventDecodeError::UnsupportedProtocolVersion);
    }
    let space = SpaceId::from_bytes(decoder.fixed().map_err(map_decode_error)?);
    let author = IdentityId::from_bytes(decoder.fixed().map_err(map_decode_error)?);
    let device = DeviceId::from_bytes(decoder.fixed().map_err(map_decode_error)?);
    let sequence = decoder.u64().map_err(map_decode_error)?;
    let timestamp_ms = decoder.i64().map_err(map_decode_error)?;

    let schema_bytes = decoder.bytes().map_err(map_decode_error)?;
    if schema_bytes.len() > MAX_EVENT_SCHEMA_BYTES {
        return Err(EventDecodeError::SchemaTooLarge);
    }
    let schema = core::str::from_utf8(schema_bytes)
        .map_err(|_| EventDecodeError::InvalidUtf8)?
        .to_owned();

    let payload = decoder.bytes().map_err(map_decode_error)?;
    if payload.len() > MAX_EVENT_PAYLOAD_BYTES {
        return Err(EventDecodeError::PayloadTooLarge);
    }

    decoder.finish().map_err(map_decode_error)?;

    Ok((
        EventHeader {
            protocol_version,
            space,
            author,
            device,
            sequence,
            timestamp_ms,
            schema,
        },
        payload.to_vec(),
    ))
}

fn derive_event_id(unsigned: &[u8], signature: &Signature) -> EventId {
    EventId::from_bytes(hash32(EVENT_ID_DOMAIN, &[unsigned, signature.as_bytes()]))
}

fn domain_wrap(domain: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + bytes.len());
    out.extend_from_slice(domain);
    out.extend_from_slice(bytes);
    out
}

fn map_encode_error(error: CodecEncodeError) -> EncodeError {
    match error {
        CodecEncodeError::FieldTooLarge => EncodeError::FieldTooLarge,
    }
}

fn map_decode_error(error: CodecDecodeError) -> EventDecodeError {
    match error {
        CodecDecodeError::UnexpectedEof => EventDecodeError::UnexpectedEof,
        CodecDecodeError::TrailingBytes => EventDecodeError::TrailingBytes,
    }
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
    fn canonical_event_round_trip() {
        let (root, device, authorization, header) = fixture();
        let event = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            b"hello".to_vec(),
        )
        .unwrap();

        let bytes = event.canonical_bytes().unwrap();
        let decoded = Event::from_canonical_bytes(&bytes).unwrap();

        assert_eq!(decoded, event);
        decoded.verify(root.document(), &authorization).unwrap();
    }

    #[test]
    fn canonical_event_rejects_trailing_bytes() {
        let (root, device, authorization, header) = fixture();
        let event = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            b"hello".to_vec(),
        )
        .unwrap();

        let mut bytes = event.canonical_bytes().unwrap();
        bytes.push(0);

        assert_eq!(
            Event::from_canonical_bytes(&bytes),
            Err(EventDecodeError::TrailingBytes)
        );
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
    fn unsupported_protocol_version_is_rejected_at_build() {
        let (root, device, authorization, mut header) = fixture();
        header.protocol_version = PROTOCOL_VERSION + 1;

        assert_eq!(
            Event::sign(
                root.document(),
                &authorization,
                &device,
                header,
                b"hello".to_vec(),
            ),
            Err(EventBuildError::UnsupportedProtocolVersion)
        );
    }

    #[test]
    fn oversized_payload_is_rejected_at_build() {
        let (root, device, authorization, header) = fixture();
        let payload = vec![0; MAX_EVENT_PAYLOAD_BYTES + 1];

        assert_eq!(
            Event::sign(root.document(), &authorization, &device, header, payload),
            Err(EventBuildError::Encode(EncodeError::PayloadTooLarge))
        );
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
