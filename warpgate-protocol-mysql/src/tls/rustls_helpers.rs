use std::io::Cursor;
use std::sync::Arc;
use std::time::SystemTime;

use rustls_pemfile;
use tokio_rustls::rustls::client::{ServerCertVerified, ServerCertVerifier, WebPkiVerifier};
use tokio_rustls::rustls::server::{ClientHello, ResolvesServerCert};
use tokio_rustls::rustls::sign::CertifiedKey;
use tokio_rustls::rustls::{Certificate, ClientConfig, Error as TlsError, ServerName};
use warpgate_common::RustlsSetupError;
use super::ROOT_CERT_STORE;

pub struct ResolveServerCert(pub Arc<CertifiedKey>);

impl ResolvesServerCert for ResolveServerCert {
    fn resolve(&self, _: ClientHello) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}

pub async fn configure_tls_connector(
    accept_invalid_certs: bool,
    accept_invalid_hostnames: bool,
    root_cert: Option<&[u8]>,
) -> Result<ClientConfig, RustlsSetupError> {
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(ROOT_CERT_STORE.clone())
        .with_no_client_auth();

    if accept_invalid_certs {
        config.dangerous().set_certificate_verifier(Arc::new(DummyTlsVerifier));
    } else {
        let mut cert_store = ROOT_CERT_STORE.clone();

        if let Some(data) = root_cert {
            let mut cursor = Cursor::new(data);

            for cert in rustls_pemfile::certs(&mut cursor)? {
                cert_store.add(&Certificate(cert))?;
            }
        }

        if accept_invalid_hostnames {
            let verifier = WebPkiVerifier::new(cert_store, None);
            config.dangerous()
                .set_certificate_verifier(Arc::new(NoHostnameTlsVerifier { verifier }));
        }
    }

    Ok(config)
}

pub struct DummyTlsVerifier;

impl ServerCertVerifier for DummyTlsVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }
}

pub struct NoHostnameTlsVerifier {
    verifier: WebPkiVerifier,
}

impl ServerCertVerifier for NoHostnameTlsVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &Certificate,
        intermediates: &[Certificate],
        server_name: &ServerName,
        scts: &mut dyn Iterator<Item = &[u8]>,
        ocsp_response: &[u8],
        now: SystemTime,
    ) -> Result<ServerCertVerified, TlsError> {
        match self.verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            scts,
            ocsp_response,
            now,
        ) {
            Err(TlsError::InvalidCertificateData(reason)) if reason.to_string().contains("CertNotValidForName") => {
            Ok(ServerCertVerified::assertion())
            }
            res => res,
        }
    }
}
