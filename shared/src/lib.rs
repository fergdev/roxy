#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod alpn;
pub mod body;
pub mod cert;
pub mod client;
pub mod content;
pub mod crypto;
pub mod h3_client;
pub mod http;
pub mod io;
pub mod tls;
pub mod uri;
pub mod util;
pub mod version;

use p12_keystore::{KeyStore, KeyStoreEntry, PrivateKeyChain};
use rand::RngCore;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_RSA_SHA256,
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use time::{Duration, OffsetDateTime};
use tracing::{debug, trace, warn};

use crate::{crypto::init_crypto, uri::RUri};

static ROXYMITM: &str = "roxymitm";
static ROXY_PWORD: &str = "roxy";

#[derive(Debug, Clone)]
pub struct RoxyCA {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    pub issuer: Issuer<'static, KeyPair>,
    pub roots: Arc<RootCertStore>,
    pub ca_der: Vec<u8>,
    pub local_leaf: LocalLeaf,
}

#[derive(Debug)]
struct LocalLeaf {
    cert_der: CertificateDer<'static>,
    pk_der: rustls::pki_types::PrivateKeyDer<'static>,
}

impl RoxyCA {
    pub fn new(
        issuer: Issuer<'static, KeyPair>,
        roots: RootCertStore,
        ca_der: Vec<u8>,
        leaf: (
            CertificateDer<'static>,
            rustls::pki_types::PrivateKeyDer<'static>,
        ),
    ) -> Self {
        let inner = Arc::new(Inner {
            issuer,
            roots: Arc::new(roots),
            ca_der,
            local_leaf: LocalLeaf {
                cert_der: leaf.0,
                pk_der: leaf.1,
            },
        });
        Self { inner }
    }

    pub fn roots(&self) -> Arc<RootCertStore> {
        self.inner.roots.clone()
    }

    pub fn sign_leaf_uri(&self, uri: &RUri) -> Result<(Certificate, KeyPair), rcgen::Error> {
        let host = uri.host();
        let mut params = CertificateParams::new(vec![host.to_string()])?;

        params.distinguished_name.push(DnType::CommonName, host);
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

        let key_pair = KeyPair::generate()?;
        let leaf = params.signed_by(&key_pair, &self.inner.issuer)?;

        Ok((leaf, key_pair))
    }

    pub fn sign_leaf_mult(
        &self,
        cn: &str,
        subject_alt_names: impl Into<Vec<String>>,
    ) -> Result<(Certificate, KeyPair), rcgen::Error> {
        let mut params = CertificateParams::new(subject_alt_names)?;

        params.distinguished_name.push(DnType::CommonName, cn);
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

        let key_pair = KeyPair::generate()?;
        let leaf = params.signed_by(&key_pair, &self.inner.issuer)?;

        Ok((leaf, key_pair))
    }

    pub fn key_pair(&self) -> &KeyPair {
        self.inner.issuer.key()
    }

    pub fn local_leaf(
        &self,
    ) -> (
        CertificateDer<'static>,
        rustls::pki_types::PrivateKeyDer<'static>,
    ) {
        (
            self.inner.local_leaf.cert_der.clone(),
            self.inner.local_leaf.pk_der.clone_key(),
        )
    }
}

fn load_native_certs(extra: Option<CertificateDer<'static>>) -> RootCertStore {
    let mut roots = rustls::RootCertStore::empty();

    let cert_result = rustls_native_certs::load_native_certs();

    for err in cert_result.errors.iter() {
        warn!("Load cert error {err}");
    }

    for cert in cert_result.certs {
        if let Err(e) = roots.add(cert) {
            warn!("failed to parse trust anchor: {}", e);
        }
    }

    if let Some(extra) = extra
        && let Err(err) = roots.add(extra)
    {
        warn!("Error adding extra cert {err}");
    }
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    roots
}

struct CaFiles {
    bundle_path_cer: PathBuf,
    bundle_path: PathBuf,
    bundle_path_ks: PathBuf,
    cert_path_cer: PathBuf,
    cert_path: PathBuf,
    cert_path_ks: PathBuf,
}

impl CaFiles {
    fn new(home: &Path) -> Self {
        let bundle_path_cer = home.join("roxy-ca.cer");
        let bundle_path = home.join("roxy-ca.pem");
        let bundle_path_ks = home.join("roxy-ca.p12");

        let cert_path_cer = home.join("roxy-ca-cert.cer");
        let cert_path = home.join("roxy-ca-cert.pem");
        let cert_path_ks = home.join("roxy-ca-cert.p12");

        CaFiles {
            bundle_path_cer,
            bundle_path,
            bundle_path_ks,
            cert_path_cer,
            cert_path,
            cert_path_ks,
        }
    }
}

#[derive(Debug)]
pub enum CaError {
    Io(std::io::Error),
    RcGen(rcgen::Error),
    KeyStore(p12_keystore::error::Error),
    RustLS(rustls::Error),
    RustLSPem(rustls::pki_types::pem::Error),
    RustLSParse,
}

impl Error for CaError {}

impl std::fmt::Display for CaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<std::io::Error> for CaError {
    fn from(value: std::io::Error) -> Self {
        CaError::Io(value)
    }
}

impl From<rcgen::Error> for CaError {
    fn from(value: rcgen::Error) -> Self {
        CaError::RcGen(value)
    }
}

impl From<p12_keystore::error::Error> for CaError {
    fn from(value: p12_keystore::error::Error) -> Self {
        CaError::KeyStore(value)
    }
}

impl From<rustls::Error> for CaError {
    fn from(value: rustls::Error) -> Self {
        CaError::RustLS(value)
    }
}
impl From<rustls::pki_types::pem::Error> for CaError {
    fn from(value: rustls::pki_types::pem::Error) -> Self {
        CaError::RustLSPem(value)
    }
}

pub fn generate_roxy_root_ca() -> Result<RoxyCA, CaError> {
    generate_roxy_root_ca_with_path(None)
}

pub fn generate_roxy_root_ca_with_path(path: Option<PathBuf>) -> Result<RoxyCA, CaError> {
    init_crypto();
    let root_dir: PathBuf = match path {
        Some(p) => p,
        None => match dirs::home_dir() {
            Some(p) => p,
            None => {
                return Err(CaError::Io(std::io::Error::other("missing home dir")));
            }
        },
    };
    let home = root_dir.join(".roxy");
    fs::create_dir_all(&home)?;

    let ca_files = CaFiles::new(&home);

    let (issuer, ca_cert) = if ca_files.bundle_path.exists() && ca_files.cert_path.exists() {
        trace!("Roxy root CA already exists at {}", home.display());
        trace!(
            "Install {} into your browser or system trust store.",
            ca_files.cert_path.display()
        );

        let pem = std::fs::read_to_string(ca_files.bundle_path.clone())?;
        let key_pair = rcgen::KeyPair::from_pem(pem.as_str())?;

        let ca_cert_pem = std::fs::read_to_string(ca_files.cert_path.clone())?;
        let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, key_pair)?;

        let ca_der = CertificateDer::from_pem_file(ca_files.bundle_path)?;

        (issuer, ca_der)
    } else {
        generate(ca_files)?
    };

    let ca_der = ca_cert.to_vec();
    let roots = load_native_certs(Some(ca_cert.clone()));
    let mut params =
        CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;

    params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    params.is_ca = IsCa::NoCa;
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

    let leaf_key_pair = KeyPair::generate()?;
    let leaf_cert = params.signed_by(&leaf_key_pair, &issuer)?;

    let leaf_kp_der =
        PrivateKeyDer::try_from(leaf_key_pair.serialize_der()).map_err(|_| CaError::RustLSParse)?;

    Ok(RoxyCA::new(
        issuer,
        roots,
        ca_der,
        (leaf_cert.der().to_owned(), leaf_kp_der),
    ))
}

fn generate(
    ca_files: CaFiles,
) -> Result<(Issuer<'static, KeyPair>, CertificateDer<'static>), CaError> {
    let mut ca_params = CertificateParams::default();
    ca_params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

    ca_params.distinguished_name = DistinguishedName::new();
    ca_params.distinguished_name.push(DnType::CountryName, "US"); // TODO: might not need this
    ca_params
        .distinguished_name
        .push(DnType::CommonName, ROXYMITM);
    ca_params
        .distinguished_name
        .push(DnType::OrganizationName, ROXYMITM);

    ca_params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    ca_params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    ca_params.key_usages.push(KeyUsagePurpose::CrlSign);

    ca_params.not_before = OffsetDateTime::now_utc();
    ca_params.not_after = OffsetDateTime::now_utc().saturating_add(Duration::days(365 * 10));

    let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA256)?;
    let ca_cert = ca_params.self_signed(&key_pair)?;

    let cert_pem = ca_cert.pem();
    let key_pem = key_pair.serialize_pem();

    let bundle = format!("{}\n{}", key_pem.trim_end(), cert_pem.trim_end());
    fs::write(&ca_files.bundle_path, bundle.clone())?;
    fs::write(&ca_files.bundle_path_cer, bundle.clone())?;

    fs::write(&ca_files.cert_path, cert_pem.clone())?;
    fs::write(&ca_files.cert_path_cer, cert_pem)?;

    let mut key_store = KeyStore::new();
    let certificate = p12_keystore::Certificate::from_der(ca_cert.der())?;

    let mut local_key_id = vec![0u8; 20];
    rand::rng().fill_bytes(&mut local_key_id);

    let key_chain =
        PrivateKeyChain::new(key_pair.serialized_der(), local_key_id, vec![certificate]);
    let key_entry = KeyStoreEntry::PrivateKeyChain(key_chain);

    key_store.add_entry(ROXYMITM, key_entry);

    let writer = key_store.writer(ROXY_PWORD);
    let data = writer.write()?;

    std::fs::write(ca_files.bundle_path_ks, data)?;

    let mut key_store = KeyStore::new();

    let mut local_key_id = vec![0u8; 20];
    rand::rng().fill_bytes(&mut local_key_id);

    let certificate = p12_keystore::Certificate::from_der(ca_cert.der())?;
    let cert_entry = KeyStoreEntry::Certificate(certificate);

    key_store.add_entry(ROXYMITM, cert_entry);

    let writer = key_store.writer(ROXY_PWORD);
    let data = writer.write()?;

    std::fs::write(ca_files.cert_path_ks, data)?;

    debug!("Roxy root CA generated:");
    debug!("Bundle path {}", ca_files.bundle_path.display());
    debug!("Cert path {}", ca_files.cert_path.display());
    debug!("");
    debug!("Import the .pem cert into your browser/system as a trusted root CA.");

    let issuer = Issuer::new(ca_params, key_pair);
    Ok((issuer, ca_cert.der().clone()))
}
