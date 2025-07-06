use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{SignatureScheme, pki_types::*};
use tracing::trace;

use rustls::pki_types::ServerName;

use crate::flow::CertInfo;

#[derive(Debug)]
pub struct LoggingCertVerifier {
    pub certs: std::sync::Mutex<Vec<CertInfo>>,
}

impl LoggingCertVerifier {
    pub fn new() -> Self {
        LoggingCertVerifier {
            certs: std::sync::Mutex::new(vec![]),
        }
    }
}

impl Default for LoggingCertVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCertVerifier for LoggingCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        trace!("Verifying server certificate for: {:?}", server_name);

        for cert in intermediates.iter() {
            self.certs
                .lock()
                .unwrap()
                .push(CertInfo::from_der(cert).unwrap());
        }

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
