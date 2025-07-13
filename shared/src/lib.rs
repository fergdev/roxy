use once_cell::sync::OnceCell;
use p12_keystore::{KeyStore, KeyStoreEntry, PrivateKeyChain};
use rand::RngCore;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_RSA_SHA256,
};
use rustls::pki_types::{CertificateDer, pem::PemObject};
use std::{fs, path::PathBuf};
use time::{Duration, OffsetDateTime};
use tracing::debug;

// TODO: rename RoxyCerts
// TODO: should this arc????? Might be required for tests
pub struct RoxyCA {
    pub issuer: Issuer<'static, KeyPair>,
    pub ca_der: CertificateDer<'static>,
}

impl RoxyCA {
    pub fn sign_leaf(&self, host: &str) -> anyhow::Result<(Certificate, KeyPair)> {
        let mut params = CertificateParams::new(vec![host.to_string()])?;

        params.distinguished_name.push(DnType::CommonName, host);
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

        let key_pair = KeyPair::generate()?;
        let leaf = params.signed_by(&key_pair, &self.issuer)?;

        Ok((leaf, key_pair))
    }

    pub fn sign_leaf_mult(
        &self,
        cn: &str,
        subject_alt_names: impl Into<Vec<String>>,
    ) -> anyhow::Result<(Certificate, KeyPair)> {
        let mut params = CertificateParams::new(subject_alt_names)?;

        params.distinguished_name.push(DnType::CommonName, cn);
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

        let key_pair = KeyPair::generate()?;
        let leaf = params.signed_by(&key_pair, &self.issuer)?;

        Ok((leaf, key_pair))
    }

    pub fn key_pair(&self) -> &KeyPair {
        self.issuer.key()
    }
}

pub fn generate_roxy_root_ca() -> anyhow::Result<RoxyCA> {
    generate_roxy_root_ca_with_path(None)
}

pub fn generate_roxy_root_ca_with_path(path: Option<PathBuf>) -> anyhow::Result<RoxyCA> {
    let root_dir: PathBuf = path.unwrap_or(dirs::home_dir().unwrap());

    // TODO: use config dir if available
    let home = root_dir.join(".roxy");
    fs::create_dir_all(&home)?;

    let bundle_path_cer = home.join("roxy-ca.cer");
    let bundle_path = home.join("roxy-ca.pem");
    let bundle_path_ks = home.join("roxy-ca.p12");

    let cert_path_cer = home.join("roxy-ca-cert.cer");
    let cert_path = home.join("roxy-ca-cert.pem");
    let cert_path_ks = home.join("roxy-ca-cert.p12");

    if bundle_path.exists() && cert_path.exists() {
        debug!("Roxy root CA already exists at {}", home.display());
        debug!(
            "Install {} into your browser or system trust store.",
            cert_path.display()
        );

        let pem = std::fs::read_to_string(bundle_path.clone()).expect("Invalid bundle path");
        let key_pair = rcgen::KeyPair::from_pem(pem.as_str()).unwrap();

        let ca_cert_pem = std::fs::read_to_string(cert_path.clone())?;

        // let issuer = Issuer::from_ca_cert_der(&ca_der, key_pair).unwrap();
        let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, key_pair).unwrap();

        let ca_der = CertificateDer::from_pem_file(bundle_path).unwrap();
        return Ok(RoxyCA { issuer, ca_der });
    }
    let mut ca_params = CertificateParams::default();
    ca_params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

    ca_params.distinguished_name = DistinguishedName::new();
    ca_params.distinguished_name.push(DnType::CountryName, "US"); // TODO: might not need this
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "roxymitm");
    ca_params
        .distinguished_name
        .push(DnType::OrganizationName, "roxymitm");

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
    fs::write(&bundle_path, bundle.clone())?;
    fs::write(&bundle_path_cer, bundle.clone())?;

    fs::write(&cert_path, cert_pem.clone())?;
    fs::write(&cert_path_cer, cert_pem)?;

    let mut key_store = KeyStore::new();
    let certificate = p12_keystore::Certificate::from_der(ca_cert.der())?;

    let mut local_key_id = vec![0u8; 20];
    rand::rng().fill_bytes(&mut local_key_id);

    let key_chain =
        PrivateKeyChain::new(key_pair.serialized_der(), local_key_id, vec![certificate]);
    let key_entry = KeyStoreEntry::PrivateKeyChain(key_chain);

    key_store.add_entry("roxymitm", key_entry);

    let writer = key_store.writer("roxy");
    let data = writer.write().unwrap();

    std::fs::write(bundle_path_ks, data)?;

    let mut key_store = KeyStore::new();

    let mut local_key_id = vec![0u8; 20];
    rand::rng().fill_bytes(&mut local_key_id);

    let certificate = p12_keystore::Certificate::from_der(ca_cert.der())?;
    let cert_entry = KeyStoreEntry::Certificate(certificate);

    key_store.add_entry("roxymitm", cert_entry);

    let writer = key_store.writer("roxy");
    let data = writer.write().unwrap();

    std::fs::write(cert_path_ks, data)?;

    debug!("Roxy root CA generated:");
    debug!("Bundle path {}", bundle_path.display());
    debug!("Cert path {}", cert_path.display());
    debug!("");
    debug!("Import the .pem cert into your browser/system as a trusted root CA.");

    Ok(RoxyCA {
        issuer: Issuer::new(ca_params, key_pair),
        ca_der: ca_cert.der().to_owned(),
    })
}

pub static INIT_CRYPTO: OnceCell<()> = OnceCell::new();

pub fn init_crypto() {
    INIT_CRYPTO.get_or_init(|| {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");
    });
}
