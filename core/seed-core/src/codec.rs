use crate::event::EventKind;
use crate::genesis::SignableGenesis;
use crate::identity::{DeviceId, IdentityId};
use crate::space::SpaceId;
use crate::{PROTOCOL_MAJOR, PROTOCOL_MINOR};

const EVENT_DOMAIN: [u8; 8] = *b"SEED-EV1";
const EVENT_FIXED_LEN: usize = 8 + 2 + 2 + 2 + 32 + 32 + 32 + 4;
const GENESIS_DOMAIN: [u8; 8] = *b"SEED-GN1";
const GENESIS_FIXED_LEN: usize = 8 + 2 + 2 + 2 + 32 + 32 + 8 + 2;
const GENESIS_PLUGIN_FIXED_LEN: usize = 16 + 2 + 2 + 2 + 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    PayloadTooLarge,
    TooManyPlugins,
    PluginConfigTooLarge,
    PluginsNotStrictlySorted,
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

pub fn encoded_genesis_len(genesis: &SignableGenesis<'_>) -> Result<usize, EncodeError> {
    u16::try_from(genesis.plugins.len()).map_err(|_| EncodeError::TooManyPlugins)?;
    validate_plugin_order(genesis)?;

    let mut required = GENESIS_FIXED_LEN;
    for plugin in genesis.plugins {
        u32::try_from(plugin.config.len()).map_err(|_| EncodeError::PluginConfigTooLarge)?;
        required = required
            .checked_add(GENESIS_PLUGIN_FIXED_LEN)
            .and_then(|size| size.checked_add(plugin.config.len()))
            .ok_or(EncodeError::PluginConfigTooLarge)?;
    }
    Ok(required)
}

/// Encode deterministic Genesis signing bytes.
///
/// Plugin entries must be strictly sorted by raw PluginId bytes. This makes
/// Genesis plugin collections deterministic and rejects duplicates.
pub fn encode_genesis_for_signing(
    genesis: &SignableGenesis<'_>,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    let plugin_count =
        u16::try_from(genesis.plugins.len()).map_err(|_| EncodeError::TooManyPlugins)?;
    let required = encoded_genesis_len(genesis)?;

    if output.len() < required {
        return Err(EncodeError::OutputTooSmall { required });
    }

    let mut cursor = 0;
    write(&mut cursor, output, &GENESIS_DOMAIN);
    write(&mut cursor, output, &PROTOCOL_MAJOR.to_be_bytes());
    write(&mut cursor, output, &PROTOCOL_MINOR.to_be_bytes());
    write(&mut cursor, output, &genesis.kind.code().to_be_bytes());
    write(&mut cursor, output, genesis.space.as_bytes());
    write(&mut cursor, output, genesis.creator.as_bytes());
    write(&mut cursor, output, &genesis.created_at_ms.to_be_bytes());
    write(&mut cursor, output, &plugin_count.to_be_bytes());

    for plugin in genesis.plugins {
        let config_len =
            u32::try_from(plugin.config.len()).map_err(|_| EncodeError::PluginConfigTooLarge)?;
        write(&mut cursor, output, plugin.id.as_bytes());
        write(&mut cursor, output, &plugin.version.major.to_be_bytes());
        write(&mut cursor, output, &plugin.version.minor.to_be_bytes());
        write(&mut cursor, output, &plugin.version.patch.to_be_bytes());
        write(&mut cursor, output, &config_len.to_be_bytes());
        write(&mut cursor, output, plugin.config);
    }

    debug_assert_eq!(cursor, required);
    Ok(required)
}

fn validate_plugin_order(genesis: &SignableGenesis<'_>) -> Result<(), EncodeError> {
    for pair in genesis.plugins.windows(2) {
        if pair[0].id.as_bytes() >= pair[1].id.as_bytes() {
            return Err(EncodeError::PluginsNotStrictlySorted);
        }
    }
    Ok(())
}

fn write(cursor: &mut usize, output: &mut [u8], bytes: &[u8]) {
    let end = *cursor + bytes.len();
    output[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
}

#[cfg(test)]
mod tests {
    use super::{
        encode_event_for_signing, encode_genesis_for_signing, encoded_event_len,
        encoded_genesis_len, EncodeError, SignableEvent,
    };
    use crate::event::EventKind;
    use crate::genesis::{GenesisPlugin, SignableGenesis};
    use crate::identity::{DeviceId, IdentityId};
    use crate::plugin::{PluginId, PluginVersion};
    use crate::space::{SpaceId, SpaceKind};

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

    fn vector_genesis<'a>(plugins: &'a [GenesisPlugin<'a>]) -> SignableGenesis<'a> {
        SignableGenesis {
            space: SpaceId::from_bytes([0x55; 32]),
            kind: SpaceKind::GROUP,
            creator: IdentityId::from_bytes([0x33; 32]),
            created_at_ms: 1_700_000_000_000,
            plugins,
        }
    }

    #[test]
    fn genesis_vector_is_stable_and_sorted() {
        let plugins = [
            GenesisPlugin {
                id: PluginId::from_bytes([0x10; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                config: b"owner=alice",
            },
            GenesisPlugin {
                id: PluginId::from_bytes([0x20; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
                config: b"",
            },
        ];
        let genesis = vector_genesis(&plugins);
        let mut output = [0u8; 151];
        let written = encode_genesis_for_signing(&genesis, &mut output).unwrap();

        assert_eq!(written, 151);
        assert_eq!(&output[0..8], b"SEED-GN1");
        assert_eq!(&output[8..10], &[0x00, 0x00]);
        assert_eq!(&output[10..12], &[0x00, 0x01]);
        assert_eq!(&output[12..14], &[0x00, 0x02]);
        assert_eq!(&output[14..46], &[0x55; 32]);
        assert_eq!(&output[46..78], &[0x33; 32]);
        assert_eq!(&output[78..86], &1_700_000_000_000u64.to_be_bytes());
        assert_eq!(&output[86..88], &[0x00, 0x02]);
        assert_eq!(&output[88..104], &[0x10; 16]);
        assert_eq!(&output[104..110], &[0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(&output[110..114], &[0x00, 0x00, 0x00, 0x0b]);
        assert_eq!(&output[114..125], b"owner=alice");
        assert_eq!(&output[125..141], &[0x20; 16]);
        assert_eq!(&output[141..147], &[0x00, 0x01, 0x00, 0x02, 0x00, 0x03]);
        assert_eq!(&output[147..151], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[cfg(feature = "crypto-ed25519")]
    #[test]
    fn genesis_bytes_have_stable_signature() {
        use crate::crypto::{verify_event_signature, EventSigningKey};

        let plugins = [
            GenesisPlugin {
                id: PluginId::from_bytes([0x10; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                config: b"owner=alice",
            },
            GenesisPlugin {
                id: PluginId::from_bytes([0x20; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
                config: b"",
            },
        ];
        let genesis = vector_genesis(&plugins);
        let mut output = [0u8; 151];
        let written = encode_genesis_for_signing(&genesis, &mut output).unwrap();

        let signing_key = EventSigningKey::from_secret_bytes(&[0x11; 32]);
        let public_key = signing_key.verifying_key_bytes();
        let signature = signing_key.sign(&output[..written]);
        let expected_signature = [
            0x6a, 0xa4, 0x26, 0xca, 0x15, 0xc9, 0xda, 0xe8, 0xa7, 0x5e, 0x95, 0x65, 0xdb, 0xcd,
            0xbe, 0x13, 0xf4, 0x76, 0x2a, 0xe4, 0x29, 0x81, 0x01, 0x11, 0x8d, 0x44, 0x42, 0x17,
            0xe7, 0xb9, 0xe7, 0xc9, 0xab, 0xae, 0x07, 0x9e, 0x09, 0x36, 0x3b, 0x75, 0xf7, 0x0c,
            0xfc, 0xb6, 0x69, 0x85, 0x28, 0x61, 0x45, 0xb5, 0xc8, 0x06, 0x00, 0x91, 0xe7, 0xf6,
            0x7f, 0x8d, 0xe5, 0x70, 0xcb, 0xfc, 0xd1, 0x04,
        ];

        assert_eq!(signature, expected_signature);
        assert!(verify_event_signature(&public_key, &output[..written], &signature).is_ok());
    }

    #[test]
    fn genesis_rejects_unsorted_or_duplicate_plugins() {
        let plugins = [
            GenesisPlugin {
                id: PluginId::from_bytes([0x20; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                config: b"",
            },
            GenesisPlugin {
                id: PluginId::from_bytes([0x10; 16]),
                version: PluginVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                config: b"",
            },
        ];
        assert_eq!(
            encoded_genesis_len(&vector_genesis(&plugins)),
            Err(EncodeError::PluginsNotStrictlySorted)
        );

        let duplicate = [plugins[0], plugins[0]];
        assert_eq!(
            encoded_genesis_len(&vector_genesis(&duplicate)),
            Err(EncodeError::PluginsNotStrictlySorted)
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
