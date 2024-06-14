use once_cell::sync::Lazy;
use tokio_rustls::rustls::RootCertStore;
use tokio_rustls::rustls::Certificate;

#[allow(clippy::expect_used)]
pub static ROOT_CERT_STORE: Lazy<RootCertStore> = Lazy::new(|| {
    let mut roots = RootCertStore::empty();
    for cert in
        rustls_native_certs::load_native_certs().expect("could not load root TLS certificates")
    {
        roots
            .add(&Certificate(cert.0))
            .expect("could not add root TLS certificate");
    }
    roots
});
