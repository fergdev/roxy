use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair};
use std::{fs, path::PathBuf};
use tracing::debug;

pub struct RoxyCA {
    pub certificate: Certificate,
    pub key_pair: KeyPair,
}

pub fn generate_roxy_root_ca() -> anyhow::Result<RoxyCA> {
    generate_roxy_root_ca_with_path(None)
}

pub fn generate_roxy_root_ca_with_path(path: Option<PathBuf>) -> anyhow::Result<RoxyCA> {
    let root_dir: PathBuf = path.unwrap_or(dirs::home_dir().unwrap());

    // TODO: use config dir if available
    let home = root_dir.join(".roxy");
    fs::create_dir_all(&home)?;

    let bundle_path = home.join("roxy-ca.pem");
    let cert_path = home.join("roxy-ca-cert.pem");

    if bundle_path.exists() && cert_path.exists() {
        debug!("Roxy root CA already exists at {}", home.display());
        debug!(
            "Install {} into your browser or system trust store.",
            cert_path.display()
        );

        let pem = std::fs::read_to_string(bundle_path.clone()).expect("Invalid bundle path");
        let key_pair = rcgen::KeyPair::from_pem(pem.as_str()).unwrap();

        let ca_cert_pem = std::fs::read_to_string(cert_path.clone())?;
        let params = CertificateParams::from_ca_cert_pem(ca_cert_pem.as_str()).unwrap();
        let certificate = params.self_signed(&key_pair).unwrap();

        return Ok(RoxyCA {
            certificate,
            key_pair,
        });
    }

    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "Roxy MITM Root CA");

    let key_pair = KeyPair::generate().unwrap();
    let cert = params.self_signed(&key_pair)?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    // Write cert + key as bundle
    let bundle = format!("{}\n{}", key_pem.trim_end(), cert_pem.trim_end());
    fs::write(&bundle_path, bundle)?;

    // Write cert only
    fs::write(&cert_path, cert_pem)?;

    debug!("Roxy root CA generated:");
    debug!("Bundle path {}", bundle_path.display());
    debug!("Cert path {}", cert_path.display());
    debug!("");
    debug!("Import the .pem cert into your browser/system as a trusted root CA.");

    Ok(RoxyCA {
        certificate: cert,
        key_pair,
    })
}
