use crate::{identity::digest, model::DEFAULT_PORT};
use anyhow::{bail, Result};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, Error, SignatureScheme,
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug)]
struct FingerprintVerifier {
    expected: Option<String>,
    observed: Arc<Mutex<Option<String>>>,
    provider: CryptoProvider,
}

impl ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let fingerprint = digest(cert.as_ref());
        if self
            .expected
            .as_ref()
            .is_some_and(|expected| expected != &fingerprint)
        {
            return Err(Error::General(
                "设备证书已变化，请重新连接并核对身份".into(),
            ));
        }
        *self.observed.lock().unwrap() = Some(fingerprint);
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            cert,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            cert,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub(crate) fn client(
    fingerprint: Option<String>,
) -> Result<(reqwest::Client, Arc<Mutex<Option<String>>>)> {
    let observed = Arc::new(Mutex::new(None));
    let verifier = FingerprintVerifier {
        expected: fingerprint,
        observed: observed.clone(),
        provider: rustls::crypto::ring::default_provider(),
    };
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(config)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(60))
        .build()?;
    Ok((client, observed))
}

pub fn normalize_address(input: &str) -> Result<String> {
    let input = input
        .trim()
        .strip_prefix("https://")
        .unwrap_or(input.trim())
        .trim_end_matches('/');
    let address: SocketAddr = if let Ok(address) = input.parse() {
        address
    } else {
        SocketAddr::new(input.parse::<IpAddr>()?, DEFAULT_PORT)
    };
    if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
        bail!("请输入有效的局域网 IP 和端口，例如 192.168.1.8:53318");
    }
    // Restrict network commands to explicit local/private addresses, never arbitrary URLs or DNS.
    let local = match address.ip() {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    };
    if !local {
        bail!("目前仅支持局域网地址");
    }
    Ok(address.to_string())
}
