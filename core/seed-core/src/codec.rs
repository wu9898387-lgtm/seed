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
    Ok(EVENT_FIXED_LEN + event.payload.len())
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
    let required = EVENT_FIXED_LEN + event.payload.len();

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
        assert_eq!(
            output,
            [
                0x53, 0x45, 0x45, 0x44, 0x2d, 0x45, 0x56, 0x31, 0x00, 0x00, 0x00, 0x01, 0x00,
                0x01, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
                0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
                0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
                0x22, 0x22, 0x22, 0x22, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
                0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x44, 0x44, 0x44,
                0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
                0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
                0x44, 0x44, 0x44, 0x00, 0x00, 0x00, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
            ]
        );
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
