use crate::model::{Device, Manifest, SignedOffer, PROTOCOL};
use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    signing_seed: String,
    cert_pem: String,
    key_pem: String,
    cert_der: String,
}

pub struct Identity {
    pub device: Device,
    pub cert_pem: String,
    pub key_pem: String,
    signing: SigningKey,
}

pub fn digest(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}

impl Identity {
    pub fn load(dir: &Path, name: Option<String>) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("identity.json");
        let stored: StoredIdentity = if path.exists() {
            serde_json::from_slice(&fs::read(&path)?).context("设备身份文件无法读取")?
        } else {
            let signing = SigningKey::generate(&mut OsRng);
            let cert = rcgen::generate_simple_self_signed(vec!["swoosh.local".into()])?;
            let value = StoredIdentity {
                signing_seed: hex::encode(signing.to_bytes()),
                cert_pem: cert.cert.pem(),
                key_pem: cert.key_pair.serialize_pem(),
                cert_der: hex::encode(cert.cert.der()),
            };
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            use std::io::Write;
            options
                .open(&path)?
                .write_all(&serde_json::to_vec(&value)?)?;
            value
        };
        let seed: [u8; 32] = hex::decode(&stored.signing_seed)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("设备密钥格式错误"))?;
        let signing = SigningKey::from_bytes(&seed);
        let public = signing.verifying_key().to_bytes();
        let device = Device {
            id: digest(public),
            name: name.unwrap_or_else(|| {
                hostname::get()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
            platform: std::env::consts::OS.into(),
            fingerprint: digest(hex::decode(&stored.cert_der)?),
            signing_key: hex::encode(public),
            protocol: PROTOCOL,
        };
        Ok(Self {
            device,
            signing,
            cert_pem: stored.cert_pem,
            key_pem: stored.key_pem,
        })
    }

    pub fn sign(&self, manifest: Manifest) -> Result<SignedOffer> {
        let signature = hex::encode(
            self.signing
                .sign(&serde_json::to_vec(&manifest)?)
                .to_bytes(),
        );
        Ok(SignedOffer {
            manifest,
            signature,
        })
    }
}

pub fn verify_offer(offer: &SignedOffer, receiver: &str) -> Result<()> {
    let manifest = &offer.manifest;
    if manifest.protocol != PROTOCOL
        || manifest.sender.protocol != PROTOCOL
        || manifest.receiver_fingerprint != receiver
    {
        bail!("协议或接收端身份不匹配");
    }
    let public: [u8; 32] = hex::decode(&manifest.sender.signing_key)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("发送端公钥格式错误"))?;
    if digest(public) != manifest.sender.id {
        bail!("发送端身份不匹配");
    }
    let signature = ed25519_dalek::Signature::from_slice(&hex::decode(&offer.signature)?)?;
    VerifyingKey::from_bytes(&public)?
        .verify_strict(&serde_json::to_vec(manifest)?, &signature)
        .context("发送请求签名无效")?;
    Ok(())
}

pub fn verification_code(manifest: &Manifest) -> Result<String> {
    let hash = Sha256::digest(serde_json::to_vec(manifest)?);
    let number = u32::from_be_bytes(hash[..4].try_into().unwrap()) % 1_000_000;
    Ok(format!("{number:06}"))
}
