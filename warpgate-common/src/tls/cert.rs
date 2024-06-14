use std::path::Path;
use std::sync::Arc;

use poem::listener::RustlsCertificate;
use rustls_pemfile;
use tokio_rustls::rustls::sign::{CertifiedKey, SigningKey, any_supported_type};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::RustlsSetupError;

pub struct TlsCertificateBundle {
    bytes: Vec<u8>,
    certificates: Vec<Certificate>,
}

pub struct TlsPrivateKey {
    bytes: Vec<u8>,
    key: Arc<dyn SigningKey>,
}

pub struct TlsCertificateAndPrivateKey {
    pub certificate: TlsCertificateBundle,
    pub private_key: TlsPrivateKey,
}

impl TlsCertificateBundle {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, RustlsSetupError> {
        let mut file = File::open(path).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, RustlsSetupError> {
        let certificates = rustls_pemfile::certs(&mut &bytes[..]).map(|mut certs| {
            certs
                .drain(..)
                .map(Certificate)
                .collect::<Vec<Certificate>>()
        })?;
        if certificates.is_empty() {
            return Err(RustlsSetupError::NoCertificates);
        }
        Ok(Self {
            bytes,
            certificates,
        })
    }
}

impl TlsPrivateKey {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, RustlsSetupError> {
        let mut file = File::open(path).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, RustlsSetupError> {
        let mut key = rustls_pemfile::pkcs8_private_keys(&mut bytes.as_slice())?
            .drain(..)
            .next()
            .map(PrivateKey);

        if key.is_none() {
            key = rustls_pemfile::rsa_private_keys(&mut bytes.as_slice())?
                .drain(..)
                .next()
                .map(PrivateKey);
        }

        let key = key.ok_or(RustlsSetupError::NoKeys)?;
        let key = any_supported_type(&key)?;

        Ok(Self { bytes, key })
    }
}

impl From<TlsCertificateBundle> for Vec<u8> {
    fn from(val: TlsCertificateBundle) -> Self {
        val.bytes
    }
}

impl From<TlsPrivateKey> for Vec<u8> {
    fn from(val: TlsPrivateKey) -> Self {
        val.bytes
    }
}

impl From<TlsCertificateAndPrivateKey> for RustlsCertificate {
    fn from(val: TlsCertificateAndPrivateKey) -> Self {
        RustlsCertificate::new()
            .cert(val.certificate)
            .key(val.private_key)
    }
}

impl From<TlsCertificateAndPrivateKey> for CertifiedKey {
    fn from(val: TlsCertificateAndPrivateKey) -> Self {
        let cert = val.certificate;
        let key = val.private_key;
        CertifiedKey {
            cert: cert.certificates,
            key: key.key,
            ocsp: None,
            sct_list: None,
        }
    }
}
