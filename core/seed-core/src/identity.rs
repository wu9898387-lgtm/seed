use crate::{
    crypto::{hash32, verify_strict, CryptoError, Ed25519Signer, PublicKey, Signature},
    id::{DeviceId, IdentityId},
};

const IDENTITY_ID_DOMAIN: &[u8] = b"seed:identity-id:v1\0";
const DEVICE_ID_DOMAIN: &[u8] = b"seed:device-id:v1\0";
const DEVICE_AUTH_DOMAIN: &[u8] = b"seed:device-authorization:v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityDocument {
    id: IdentityId,
    root_public_key: PublicKey,
}

impl IdentityDocument {
    pub fn from_root_public_key(root_public_key: PublicKey) -> Self {
        let id = IdentityId::from_bytes(hash32(IDENTITY_ID_DOMAIN, &[root_public_key.as_bytes()]));
        Self {
            id,
            root_public_key,
        }
    }

    pub const fn id(&self) -> IdentityId {
        self.id
    }

    pub const fn root_public_key(&self) -> PublicKey {
        self.root_public_key
    }

    pub fn fingerprint(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Debug)]
pub struct RootIdentity {
    signer: Ed25519Signer,
    document: IdentityDocument,
}

impl RootIdentity {
    pub fn generate() -> Result<Self, CryptoError> {
        let signer = Ed25519Signer::generate()?;
        Ok(Self::from_signer(signer))
    }

    pub fn from_secret_bytes(secret: [u8; 32]) -> Self {
        Self::from_signer(Ed25519Signer::from_secret_bytes(secret))
    }

    fn from_signer(signer: Ed25519Signer) -> Self {
        let document = IdentityDocument::from_root_public_key(signer.public_key());
        Self { signer, document }
    }

    pub const fn document(&self) -> &IdentityDocument {
        &self.document
    }

    pub fn authorize_device(
        &self,
        device: &DeviceIdentity,
        sequence: u64,
        issued_at_ms: i64,
    ) -> DeviceAuthorization {
        let mut authorization = DeviceAuthorization {
            identity_id: self.document.id(),
            device_id: device.id(),
            device_public_key: device.public_key(),
            sequence,
            issued_at_ms,
            root_signature: Signature::from_bytes([0u8; 64]),
        };
        authorization.root_signature = self.signer.sign(&authorization.signing_bytes());
        authorization
    }
}

#[derive(Debug)]
pub struct DeviceIdentity {
    signer: Ed25519Signer,
    id: DeviceId,
}

impl DeviceIdentity {
    pub fn generate() -> Result<Self, CryptoError> {
        let signer = Ed25519Signer::generate()?;
        Ok(Self::from_signer(signer))
    }

    pub fn from_secret_bytes(secret: [u8; 32]) -> Self {
        Self::from_signer(Ed25519Signer::from_secret_bytes(secret))
    }

    fn from_signer(signer: Ed25519Signer) -> Self {
        let id = derive_device_id(&signer.public_key());
        Self { signer, id }
    }

    pub const fn id(&self) -> DeviceId {
        self.id
    }

    pub fn public_key(&self) -> PublicKey {
        self.signer.public_key()
    }

    pub(crate) fn sign(&self, message: &[u8]) -> Signature {
        self.signer.sign(message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceAuthorization {
    identity_id: IdentityId,
    device_id: DeviceId,
    device_public_key: PublicKey,
    sequence: u64,
    issued_at_ms: i64,
    root_signature: Signature,
}

impl DeviceAuthorization {
    pub const fn identity_id(&self) -> IdentityId {
        self.identity_id
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub const fn device_public_key(&self) -> PublicKey {
        self.device_public_key
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn issued_at_ms(&self) -> i64 {
        self.issued_at_ms
    }

    pub const fn root_signature(&self) -> Signature {
        self.root_signature
    }

    pub fn verify_against(&self, identity: &IdentityDocument) -> Result<(), IdentityError> {
        if identity.id() != self.identity_id {
            return Err(IdentityError::IdentityMismatch);
        }

        let expected_identity =
            IdentityDocument::from_root_public_key(identity.root_public_key()).id();
        if expected_identity != identity.id() {
            return Err(IdentityError::InvalidIdentityDocument);
        }

        if derive_device_id(&self.device_public_key) != self.device_id {
            return Err(IdentityError::DeviceMismatch);
        }

        verify_strict(
            &identity.root_public_key(),
            &self.signing_bytes(),
            &self.root_signature,
        )
        .map_err(IdentityError::Crypto)
    }

    fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(DEVICE_AUTH_DOMAIN.len() + 112);
        out.extend_from_slice(DEVICE_AUTH_DOMAIN);
        out.extend_from_slice(self.identity_id.as_bytes());
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(self.device_public_key.as_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.issued_at_ms.to_be_bytes());
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityError {
    Crypto(CryptoError),
    IdentityMismatch,
    DeviceMismatch,
    InvalidIdentityDocument,
}

fn derive_device_id(public_key: &PublicKey) -> DeviceId {
    DeviceId::from_bytes(hash32(DEVICE_ID_DOMAIN, &[public_key.as_bytes()]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_id_is_derived_from_root_public_key() {
        let root = RootIdentity::generate().unwrap();
        let rebuilt = IdentityDocument::from_root_public_key(root.document().root_public_key());

        assert_eq!(root.document().id(), rebuilt.id());
    }

    #[test]
    fn root_authorizes_device() {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 123);

        authorization.verify_against(root.document()).unwrap();
        assert_eq!(authorization.identity_id(), root.document().id());
        assert_eq!(authorization.device_id(), device.id());
    }

    #[test]
    fn authorization_is_bound_to_one_identity() {
        let root = RootIdentity::generate().unwrap();
        let other_root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 123);

        assert_eq!(
            authorization.verify_against(other_root.document()),
            Err(IdentityError::IdentityMismatch)
        );
    }
}
