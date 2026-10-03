use crate::event::EventKind;
use crate::identity::{DeviceId, IdentityId};
use crate::space::SpaceId;
use crate::{PROTOCOL_MAJOR, PROTOCOL_MINOR};

const EVENT_DOMAIN: [u8; 8] = *b"SEED-EV1";
const EVENT_FIXED_LEN: usize = 8 + 2 + 2 + 2 + 32 + 32 + 32 + 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    PayloadTooLarge,
    OutputTooSmall { required: usize },
}

/// Fields covered by an Event signature.
///
/// EventId and the signature itself are intentionally excluded: both are
/// derived from, or attest to, these canonical bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignableEvent<'a> {
    pub space: SpaceId,
    pub author: IdentityId,
    pub device: DeviceId,
    pub kind: EventKind,
    pub payload: &'a [u8],
}

pub fn encoded_event_len(event: &SignableEvent<'_>) -> Result<usize, EncodeError> {
    u32::try_from(event.payload.len()).map_err(|_| EncodeError::PayloadTooLarge)?;
    EVENT_FIXED_LEN
        .checked_add(event.payload.len())
        .ok_or(EncodeError::PayloadTooLarge)
}

/// Encode the Phase 0 Event signing preimage.
///
/// This is a deliberately narrow protocol-owned framing spike, not a final
/// general serialization decision. All integers are big-endian, lengths are
/// explicit, fields are ordered, and there are no maps or optional fields.
pub fn encode_event_for_signing(
    event: &SignableEvent<'_>,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    let payload_len =
        u32::try_from(event.payload.len()).map_err(|_| EncodeError::PayloadTooLarge)?;
    let required = EVENT_FIXED_LEN
        .checked_add(event.payload.len())
        .ok_or(EncodeError::PayloadTooLarge)?;

    if output.len() < required {
        return Err(EncodeError::OutputTooSmall { required });
    }

    let mut cursor = 0;

    write(&mut cursor, output, &EVENT_DOMAIN);
    write(&mut cursor, output, &PROTOCOL_MAJOR.to_be_bytes());
    write(&mut cursor, output, &PROTOCOL_MINOR.to_be_bytes());
    write(&mut cursor, output, &event.kind.code().to_be_bytes());
    write(&mut cursor, output, event.space.as_bytes());
    write(&mut cursor, output, event.author.as_bytes());
    write(&mut cursor, output, event.device.as_bytes());
    write(&mut cursor, output, &payload_len.to_be_bytes());
    write(&mut cursor, output, event.payload);

    debug_assert_eq!(cursor, required);
    Ok(required)
}

fn write(cursor: &mut usize, output: &mut [u8], bytes: &[u8]) {
    let end = *cursor + bytes.len();
    output[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
}

#[cfg(test)]
mod tests {
    use super::{encode_event_for_signing, encoded_event_len, EncodeError, SignableEvent};
    use crate::event::EventKind;
    use crate::identity::{DeviceId, IdentityId};
    use crate::space::SpaceId;

    const PAYLOAD: &[u8] = b"hello";

    fn vector_event() -> SignableEvent<'static> {
        SignableEvent {
            space: SpaceId::from_bytes([0x22; 32]),
            author: IdentityId::from_bytes([0x33; 32]),
            device: DeviceId::from_bytes([0x44; 32]),
            kind: EventKind::MESSAGE,
            payload: PAYLOAD,
        }
    }

    #[test]
    fn canonical_vector_is_stable() {
        let event = vector_event();
        let mut output = [0u8; 119];
        let written = encode_event_for_signing(&event, &mut output).unwrap();

        assert_eq!(written, 119);
        assert_eq!(&output[0..8], b"SEED-EV1");
        assert_eq!(&output[8..10], &[0x00, 0x00]);
        assert_eq!(&output[10..12], &[0x00, 0x01]);
        assert_eq!(&output[12..14], &[0x00, 0x01]);
        assert_eq!(&output[14..46], &[0x22; 32]);
        assert_eq!(&output[46..78], &[0x33; 32]);
        assert_eq!(&output[78..110], &[0x44; 32]);
        assert_eq!(&output[110..114], &[0x00, 0x00, 0x00, 0x05]);
        assert_eq!(&output[114..119], b"hello");
    }

    #[cfg(feature = "crypto-ed25519")]
    #[test]
    fn canonical_bytes_have_stable_signature() {
        use crate::crypto::{verify_event_signature, EventSigningKey};

        let event = vector_event();
        let mut output = [0u8; 119];
        let written = encode_event_for_signing(&event, &mut output).unwrap();

        let signing_key = EventSigningKey::from_secret_bytes(&[0x11; 32]);
        let public_key = signing_key.verifying_key_bytes();
        let signature = signing_key.sign(&output[..written]);
        let expected_signature = [
            0x91, 0xd4, 0xc5, 0xb6, 0x74, 0xb0, 0x91, 0x38, 0x0a, 0x2a, 0xb2, 0x29, 0x35, 0x73,
            0x64, 0xfe, 0x3a, 0x4e, 0x03, 0x78, 0x80, 0x38, 0xeb, 0x32, 0xd5, 0xff, 0x1d, 0xdf,
            0x79, 0x02, 0x2c, 0xa9, 0xc0, 0xe0, 0xc9, 0x80, 0x2f, 0x4e, 0x78, 0x38, 0xe7, 0x52,
            0xbf, 0x6b, 0xcb, 0xe2, 0x77, 0x38, 0x3e, 0xf3, 0x82, 0x6d, 0x66, 0x28, 0x07, 0x7e,
            0x4f, 0x69, 0x6a, 0x56, 0x8f, 0xd7, 0xd8, 0x06,
        ];

        assert_eq!(signature, expected_signature);
        assert!(verify_event_signature(&public_key, &output[..written], &signature).is_ok());
    }

    #[test]
    fn reports_required_output_size() {
        let event = vector_event();
        assert_eq!(encoded_event_len(&event), Ok(119));

        let mut output = [0u8; 118];
        assert_eq!(
            encode_event_for_signing(&event, &mut output),
            Err(EncodeError::OutputTooSmall { required: 119 })
        );
    }
}
