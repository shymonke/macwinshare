//! TLS 1.3 encryption using rustls
//! 
//! Generates self-signed certificates on first run and handles
//! certificate fingerprint verification for peer authentication.

use crate::{Config, Error, Result};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Manages TLS certificates and configurations
pub struct TlsManager {
    certificate: CertificateDer<'static>,
    private_key: PrivateKeyDer<'static>,
    fingerprint: String,
}

impl TlsManager {
    /// Create a new TLS manager, loading or generating certificates
    pub fn new(config: &Config) -> Result<Self> {
        let cert_dir = config.data_dir.join("ssl");
        fs::create_dir_all(&cert_dir)?;

        let cert_path = cert_dir.join("certificate.pem");
        let key_path = cert_dir.join("private_key.pem");

        let (certificate, private_key) = if cert_path.exists() && key_path.exists() {
            info!("Loading existing certificates from {:?}", cert_dir);
            Self::load_certificates(&cert_path, &key_path)?
        } else {
            info!("Generating new self-signed certificate");
            Self::generate_certificates(&cert_path, &key_path, &config.machine_name)?
        };

        let fingerprint = Self::calculate_fingerprint(&certificate);
        info!("Certificate fingerprint: {}", fingerprint);

        Ok(Self {
            certificate,
            private_key,
            fingerprint,
        })
    }

    /// Get the certificate fingerprint for verification
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Create a rustls ServerConfig for accepting connections
    pub fn server_config(&self) -> Result<Arc<ServerConfig>> {
        let certs = vec![self.certificate.clone()];
        let key = self.private_key.clone_key();

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Tls(e))?;

        Ok(Arc::new(config))
    }

    /// Create a rustls ClientConfig for connecting to servers
    pub fn client_config(&self) -> Result<Arc<ClientConfig>> {
        // For now, we accept any certificate and verify fingerprint manually
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCertVerifier))
            .with_no_client_auth();

        Ok(Arc::new(config))
    }

    /// Verify a peer's certificate fingerprint
    pub fn verify_fingerprint(cert: &CertificateDer, expected: &str) -> bool {
        let actual = Self::calculate_fingerprint(cert);
        actual.eq_ignore_ascii_case(expected)
    }

    fn calculate_fingerprint(cert: &CertificateDer) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cert.as_ref());
        let result = hasher.finalize();
        
        result
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    }

    fn generate_certificates(
        cert_path: &Path,
        key_path: &Path,
        machine_name: &str,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
        // Generate key pair
        let key_pair = KeyPair::generate()
            .map_err(|e| Error::Certificate(e.to_string()))?;

        // Create certificate parameters
        let mut params = CertificateParams::default();
        
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, machine_name);
        dn.push(DnType::OrganizationName, "MacWinShare");
        params.distinguished_name = dn;

        // Add SANs for local hostname and IPs
        params.subject_alt_names = vec![
            SanType::DnsName(machine_name.try_into().unwrap_or_else(|_| "localhost".try_into().unwrap())),
            SanType::DnsName("localhost".try_into().unwrap()),
        ];

        // Generate certificate
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| Error::Certificate(e.to_string()))?;

        // Save to disk
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        
        fs::write(cert_path, &cert_pem)?;
        fs::write(key_path, &key_pem)?;

        debug!("Saved certificate to {:?}", cert_path);
        debug!("Saved private key to {:?}", key_path);

        // Convert to DER format for rustls
        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

        Ok((cert_der, key_der))
    }

    fn load_certificates(
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
        let cert_pem = fs::read_to_string(cert_path)?;
        let key_pem = fs::read_to_string(key_path)?;

        // Parse PEM certificate
        let cert_der = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .next()
            .ok_or_else(|| Error::Certificate("No certificate found in PEM file".into()))?
            .map_err(|e| Error::Certificate(e.to_string()))?;

        // Parse PEM private key
        let key_der = rustls_pemfile::private_key(&mut key_pem.as_bytes())
            .map_err(|e| Error::Certificate(e.to_string()))?
            .ok_or_else(|| Error::Certificate("No private key found in PEM file".into()))?;

        Ok((cert_der, key_der))
    }
}

/// Certificate verifier that accepts any certificate
/// We do manual fingerprint verification after connection
#[derive(Debug)]
struct AcceptAnyCertVerifier;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // Accept all certificates - we verify fingerprint manually
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
