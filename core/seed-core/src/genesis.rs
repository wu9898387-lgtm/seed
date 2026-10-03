use crate::{
    codec::{DecodeError, Decoder, EncodeError, Encoder},
    crypto::{hash32, verify_strict, CryptoError, Signature},
    id::{DeviceId, GenesisId, IdentityId, PluginDigest, PluginId, SpaceId},
    identity::{DeviceAuthorization, DeviceIdentity, IdentityDocument, IdentityError},
    plugin::PluginVersion,
    space::{SpaceDescriptor, SpaceKind},
    PROTOCOL_VERSION,
};

const GENESIS_SCHEMA_VERSION: u16 = 2;
const GENESIS_WIRE_VERSION: u16 = 2;
const GENESIS_NONCE_LENGTH: usize = 16;
const GENESIS_WIRE_MAGIC: &[u8; 4] = b"SGEN";

const GENESIS_SIGNATURE_DOMAIN: &[u8] = b"seed:genesis-signature:v1\0";
const GENESIS_ID_DOMAIN: &[u8] = b"seed:genesis-id:v1\0";
const SPACE_ID_DOMAIN: &[u8] = b"seed:space-id:v1\0";

const MAX_GENESIS_PLUGINS: usize = 1024;
const MAX_PLUGIN_CONFIG_BYTES: usize = 1024 * 1024;
const MAX_GENESIS_BODY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenesisPlugin {
    id: PluginId,
    version: PluginVersion,
    package_digest: PluginDigest,
    config: Vec<u8>,
}

impl GenesisPlugin {
    pub fn new(
        id: PluginId,
        version: PluginVersion,
        package_digest: PluginDigest,
        config: Vec<u8>,
    ) -> Result<Self, GenesisBuildError> {
        if config.len() > MAX_PLUGIN_CONFIG_BYTES {
            return Err(GenesisBuildError::PluginConfigTooLarge);
        }

        Ok(Self {
            id,
            version,
            package_digest,
            config,
        })
    }

    pub const fn id(&self) -> PluginId {
        self.id
    }

    pub const fn version(&self) -> PluginVersion {
        self.version
    }

    pub const fn package_digest(&self) -> PluginDigest {
        self.package_digest
    }

    pub fn config(&self) -> &[u8] {
        &self.config
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenesisDraft {
    kind: SpaceKind,
    creator: IdentityId,
    creator_device: DeviceId,
    created_at_ms: i64,
    creation_nonce: [u8; GENESIS_NONCE_LENGTH],
    plugins: Vec<GenesisPlugin>,
}

impl GenesisDraft {
    pub fn new(
        kind: SpaceKind,
        creator: &IdentityDocument,
        device: &DeviceIdentity,
        created_at_ms: i64,
        creation_nonce: [u8; GENESIS_NONCE_LENGTH],
    ) -> Self {
        Self {
            kind,
            creator: creator.id(),
            creator_device: device.id(),
            created_at_ms,
            creation_nonce,
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: GenesisPlugin) -> Result<(), GenesisBuildError> {
        if self.plugins.len() >= MAX_GENESIS_PLUGINS {
            return Err(GenesisBuildError::TooManyPlugins);
        }

        if self.plugins.iter().any(|existing| existing.id == plugin.id) {
            return Err(GenesisBuildError::DuplicatePlugin);
        }

        self.plugins.push(plugin);
        Ok(())
    }

    pub fn activate(
        mut self,
        identity: &IdentityDocument,
        authorization: &DeviceAuthorization,
        device: &DeviceIdentity,
    ) -> Result<GenesisRecord, GenesisBuildError> {
        authorization
            .verify_against(identity)
            .map_err(GenesisBuildError::Identity)?;

        if self.creator != identity.id()
            || self.creator_device != authorization.device_id()
            || self.creator_device != device.id()
        {
            return Err(GenesisBuildError::ActorMismatch);
        }

        self.plugins.sort_by_key(|plugin| plugin.id);

        let unsigned = self
            .canonical_unsigned_bytes()
            .map_err(|_| GenesisBuildError::Encoding)?;
        let signature_input = domain_wrap(GENESIS_SIGNATURE_DOMAIN, &unsigned);
        let signature = device.sign(&signature_input);

        Ok(GenesisRecord::from_parts(self, signature))
    }

    fn canonical_unsigned_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoder = Encoder::with_capacity(160);
        encoder.u16(GENESIS_SCHEMA_VERSION);
        encoder.u16(PROTOCOL_VERSION);
        encoder.u8(self.kind.wire_value());
        encoder.fixed(self.creator.as_bytes());
        encoder.fixed(self.creator_device.as_bytes());
        encoder.i64(self.created_at_ms);
        encoder.fixed(&self.creation_nonce);

        let plugin_count =
            u16::try_from(self.plugins.len()).map_err(|_| EncodeError::FieldTooLarge)?;
        encoder.u16(plugin_count);

        for plugin in &self.plugins {
            encoder.fixed(plugin.id.as_bytes());
            encoder.u16(plugin.version.major);
            encoder.u16(plugin.version.minor);
            encoder.u16(plugin.version.patch);
            encoder.fixed(plugin.package_digest.as_bytes());
            encoder.bytes(&plugin.config)?;
        }

        Ok(encoder.finish())
    }

    fn decode_unsigned(bytes: &[u8]) -> Result<Self, GenesisDecodeError> {
        if bytes.len() > MAX_GENESIS_BODY_BYTES {
            return Err(GenesisDecodeError::BodyTooLarge);
        }

        let mut decoder = Decoder::new(bytes);

        let schema_version = decoder.u16().map_err(map_decode_error)?;
        if schema_version != GENESIS_SCHEMA_VERSION {
            return Err(GenesisDecodeError::UnsupportedSchemaVersion);
        }

        let protocol_version = decoder.u16().map_err(map_decode_error)?;
        if protocol_version != PROTOCOL_VERSION {
            return Err(GenesisDecodeError::UnsupportedProtocolVersion);
        }

        let kind = SpaceKind::from_wire(decoder.u8().map_err(map_decode_error)?)
            .ok_or(GenesisDecodeError::InvalidSpaceKind)?;

        let creator = IdentityId::from_bytes(decoder.fixed().map_err(map_decode_error)?);
        let creator_device = DeviceId::from_bytes(decoder.fixed().map_err(map_decode_error)?);
        let created_at_ms = decoder.i64().map_err(map_decode_error)?;
        let creation_nonce = decoder.fixed().map_err(map_decode_error)?;

        let plugin_count = decoder.u16().map_err(map_decode_error)? as usize;
        if plugin_count > MAX_GENESIS_PLUGINS {
            return Err(GenesisDecodeError::TooManyPlugins);
        }

        let mut plugins = Vec::with_capacity(plugin_count);
        let mut previous_id: Option<PluginId> = None;

        for _ in 0..plugin_count {
            let id = PluginId::from_bytes(decoder.fixed().map_err(map_decode_error)?);

            if let Some(previous) = previous_id {
                if id == previous {
                    return Err(GenesisDecodeError::DuplicatePlugin);
                }
                if id < previous {
                    return Err(GenesisDecodeError::NonCanonicalPluginOrder);
                }
            }
            previous_id = Some(id);

            let version = PluginVersion {
                major: decoder.u16().map_err(map_decode_error)?,
                minor: decoder.u16().map_err(map_decode_error)?,
                patch: decoder.u16().map_err(map_decode_error)?,
            };
            let package_digest =
                PluginDigest::from_bytes(decoder.fixed().map_err(map_decode_error)?);
            let config = decoder.bytes().map_err(map_decode_error)?;
            if config.len() > MAX_PLUGIN_CONFIG_BYTES {
                return Err(GenesisDecodeError::PluginConfigTooLarge);
            }

            plugins.push(GenesisPlugin {
                id,
                version,
                package_digest,
                config: config.to_vec(),
            });
        }

        decoder.finish().map_err(map_decode_error)?;

        Ok(Self {
            kind,
            creator,
            creator_device,
            created_at_ms,
            creation_nonce,
            plugins,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenesisRecord {
    id: GenesisId,
    space_id: SpaceId,
    draft: GenesisDraft,
    signature: Signature,
}

impl GenesisRecord {
    fn from_parts(draft: GenesisDraft, signature: Signature) -> Self {
        let unsigned = draft
            .canonical_unsigned_bytes()
            .expect("validated Genesis fields must encode");
        let id = GenesisId::from_bytes(hash32(
            GENESIS_ID_DOMAIN,
            &[&unsigned, signature.as_bytes()],
        ));
        let space_id = SpaceId::from_bytes(hash32(SPACE_ID_DOMAIN, &[id.as_bytes()]));

        Self {
            id,
            space_id,
            draft,
            signature,
        }
    }

    pub const fn id(&self) -> GenesisId {
        self.id
    }

    pub const fn space_id(&self) -> SpaceId {
        self.space_id
    }

    pub const fn kind(&self) -> SpaceKind {
        self.draft.kind
    }

    pub const fn creator(&self) -> IdentityId {
        self.draft.creator
    }

    pub const fn creator_device(&self) -> DeviceId {
        self.draft.creator_device
    }

    pub const fn created_at_ms(&self) -> i64 {
        self.draft.created_at_ms
    }

    pub const fn creation_nonce(&self) -> [u8; GENESIS_NONCE_LENGTH] {
        self.draft.creation_nonce
    }

    pub fn plugins(&self) -> &[GenesisPlugin] {
        &self.draft.plugins
    }

    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn descriptor(&self) -> SpaceDescriptor {
        SpaceDescriptor {
            id: self.space_id,
            kind: self.draft.kind,
            genesis: Some(self.id),
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, GenesisEncodeError> {
        let unsigned = self
            .draft
            .canonical_unsigned_bytes()
            .map_err(map_encode_error)?;

        if unsigned.len() > MAX_GENESIS_BODY_BYTES {
            return Err(GenesisEncodeError::BodyTooLarge);
        }

        let mut encoder = Encoder::with_capacity(4 + 2 + 4 + unsigned.len() + 64);
        encoder.fixed(GENESIS_WIRE_MAGIC);
        encoder.u16(GENESIS_WIRE_VERSION);
        encoder.bytes(&unsigned).map_err(map_encode_error)?;
        encoder.fixed(self.signature.as_bytes());
        Ok(encoder.finish())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, GenesisDecodeError> {
        let mut decoder = Decoder::new(bytes);
        let magic: [u8; 4] = decoder.fixed().map_err(map_decode_error)?;
        if &magic != GENESIS_WIRE_MAGIC {
            return Err(GenesisDecodeError::InvalidMagic);
        }

        let wire_version = decoder.u16().map_err(map_decode_error)?;
        if wire_version != GENESIS_WIRE_VERSION {
            return Err(GenesisDecodeError::UnsupportedWireVersion);
        }

        let unsigned = decoder.bytes().map_err(map_decode_error)?;
        if unsigned.len() > MAX_GENESIS_BODY_BYTES {
            return Err(GenesisDecodeError::BodyTooLarge);
        }

        let signature = Signature::from_bytes(decoder.fixed().map_err(map_decode_error)?);
        decoder.finish().map_err(map_decode_error)?;

        let draft = GenesisDraft::decode_unsigned(unsigned)?;
        let record = Self::from_parts(draft, signature);

        let reencoded = record
            .canonical_bytes()
            .map_err(|_| GenesisDecodeError::NonCanonicalEncoding)?;
        if reencoded != bytes {
            return Err(GenesisDecodeError::NonCanonicalEncoding);
        }

        Ok(record)
    }

    pub fn verify(
        &self,
        identity: &IdentityDocument,
        authorization: &DeviceAuthorization,
    ) -> Result<(), GenesisVerifyError> {
        authorization
            .verify_against(identity)
            .map_err(GenesisVerifyError::Identity)?;

        if self.creator() != identity.id() || self.creator_device() != authorization.device_id() {
            return Err(GenesisVerifyError::ActorMismatch);
        }

        let unsigned = self
            .draft
            .canonical_unsigned_bytes()
            .map_err(|_| GenesisVerifyError::Encoding)?;

        let expected_id = GenesisId::from_bytes(hash32(
            GENESIS_ID_DOMAIN,
            &[&unsigned, self.signature.as_bytes()],
        ));
        if expected_id != self.id {
            return Err(GenesisVerifyError::IdMismatch);
        }

        let expected_space = SpaceId::from_bytes(hash32(SPACE_ID_DOMAIN, &[self.id.as_bytes()]));
        if expected_space != self.space_id {
            return Err(GenesisVerifyError::SpaceIdMismatch);
        }

        let signature_input = domain_wrap(GENESIS_SIGNATURE_DOMAIN, &unsigned);
        verify_strict(
            &authorization.device_public_key(),
            &signature_input,
            &self.signature,
        )
        .map_err(GenesisVerifyError::Crypto)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenesisBuildError {
    Identity(IdentityError),
    Encoding,
    ActorMismatch,
    DuplicatePlugin,
    TooManyPlugins,
    PluginConfigTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenesisEncodeError {
    FieldTooLarge,
    BodyTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenesisDecodeError {
    UnexpectedEof,
    TrailingBytes,
    InvalidMagic,
    UnsupportedWireVersion,
    UnsupportedSchemaVersion,
    UnsupportedProtocolVersion,
    InvalidSpaceKind,
    DuplicatePlugin,
    NonCanonicalPluginOrder,
    TooManyPlugins,
    PluginConfigTooLarge,
    BodyTooLarge,
    NonCanonicalEncoding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenesisVerifyError {
    Identity(IdentityError),
    Encoding,
    ActorMismatch,
    IdMismatch,
    SpaceIdMismatch,
    Crypto(CryptoError),
}

fn map_encode_error(error: EncodeError) -> GenesisEncodeError {
    match error {
        EncodeError::FieldTooLarge => GenesisEncodeError::FieldTooLarge,
    }
}

fn map_decode_error(error: DecodeError) -> GenesisDecodeError {
    match error {
        DecodeError::UnexpectedEof => GenesisDecodeError::UnexpectedEof,
        DecodeError::TrailingBytes => GenesisDecodeError::TrailingBytes,
    }
}

fn domain_wrap(domain: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(domain.len() + bytes.len());
    out.extend_from_slice(domain);
    out.extend_from_slice(bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{DeviceIdentity, RootIdentity};

    fn plugin(id: u8, config: &[u8]) -> GenesisPlugin {
        GenesisPlugin::new(
            PluginId::from_bytes([id; 32]),
            PluginVersion {
                major: 1,
                minor: id as u16,
                patch: 0,
            },
            PluginDigest::from_bytes([id.wrapping_add(20); 32]),
            config.to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn genesis_round_trip_and_verify() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 10);

        let mut draft = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16]);
        draft.add_plugin(plugin(2, b"two")).unwrap();
        draft.add_plugin(plugin(1, b"one")).unwrap();

        let record = draft
            .activate(root.document(), &authorization, &device)
            .unwrap();
        let encoded = record.canonical_bytes().unwrap();
        let decoded = GenesisRecord::from_canonical_bytes(&encoded).unwrap();

        assert_eq!(decoded, record);
        assert_eq!(decoded.plugins()[0].id(), PluginId::from_bytes([1; 32]));
        decoded.verify(root.document(), &authorization).unwrap();
    }

    #[test]
    fn plugin_insertion_order_does_not_change_genesis_id() {
        let root = RootIdentity::from_secret_bytes([1; 32]);
        let device = DeviceIdentity::from_secret_bytes([2; 32]);
        let authorization = root.authorize_device(&device, 1, 10);

        let mut a = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16]);
        a.add_plugin(plugin(2, b"two")).unwrap();
        a.add_plugin(plugin(1, b"one")).unwrap();

        let mut b = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16]);
        b.add_plugin(plugin(1, b"one")).unwrap();
        b.add_plugin(plugin(2, b"two")).unwrap();

        let a = a
            .activate(root.document(), &authorization, &device)
            .unwrap();
        let b = b
            .activate(root.document(), &authorization, &device)
            .unwrap();

        assert_eq!(a.id(), b.id());
        assert_eq!(a.space_id(), b.space_id());
        assert_eq!(a.canonical_bytes().unwrap(), b.canonical_bytes().unwrap());
    }

    #[test]
    fn duplicate_plugin_is_rejected() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let mut draft = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16]);

        draft.add_plugin(plugin(1, b"one")).unwrap();
        assert_eq!(
            draft.add_plugin(plugin(1, b"other")),
            Err(GenesisBuildError::DuplicatePlugin)
        );
    }

    #[test]
    fn trailing_wire_bytes_are_rejected() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 10);
        let record = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16])
            .activate(root.document(), &authorization, &device)
            .unwrap();

        let mut encoded = record.canonical_bytes().unwrap();
        encoded.push(0);

        assert_eq!(
            GenesisRecord::from_canonical_bytes(&encoded),
            Err(GenesisDecodeError::TrailingBytes)
        );
    }

    #[test]
    fn tampered_genesis_body_fails_signature_verification() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 10);

        let mut draft = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16]);
        draft.add_plugin(plugin(1, b"one")).unwrap();
        let record = draft
            .activate(root.document(), &authorization, &device)
            .unwrap();

        let mut encoded = record.canonical_bytes().unwrap();
        let config_byte = encoded
            .iter()
            .position(|byte| *byte == b'o')
            .expect("config byte");
        encoded[config_byte] = b'x';

        let tampered = GenesisRecord::from_canonical_bytes(&encoded).unwrap();
        assert_eq!(
            tampered.verify(root.document(), &authorization),
            Err(GenesisVerifyError::Crypto(CryptoError::InvalidSignature))
        );
    }

    #[test]
    fn creation_nonce_changes_space_identity() {
        let root = RootIdentity::from_secret_bytes([1; 32]);
        let device = DeviceIdentity::from_secret_bytes([2; 32]);
        let authorization = root.authorize_device(&device, 1, 10);

        let a = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16])
            .activate(root.document(), &authorization, &device)
            .unwrap();
        let b = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [4; 16])
            .activate(root.document(), &authorization, &device)
            .unwrap();

        assert_ne!(a.id(), b.id());
        assert_ne!(a.space_id(), b.space_id());
    }

    #[test]
    fn creator_is_provenance_not_owner_state() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 10);
        let record = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 11, [3; 16])
            .activate(root.document(), &authorization, &device)
            .unwrap();

        assert_eq!(record.creator(), root.document().id());
        assert!(record.plugins().is_empty());
    }
}
