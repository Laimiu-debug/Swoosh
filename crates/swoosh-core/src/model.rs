use serde::{Deserialize, Serialize};

pub const PROTOCOL: u32 = 1;
pub const SERVICE: &str = "_swoosh._tcp.local.";
pub const TEXT_LIMIT: u64 = 1024 * 1024;
pub const ENTRY_LIMIT: usize = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub fingerprint: String,
    pub signing_key: String,
    pub protocol: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub device: Device,
    pub address: String,
    pub discovered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    Files,
    Text,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub directory: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub id: String,
    pub title: String,
    pub count: usize,
    pub total_bytes: u64,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub protocol: u32,
    pub nonce: String,
    pub sender: Device,
    pub receiver_fingerprint: String,
    pub kind: ContentKind,
    pub title: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedOffer {
    pub manifest: Manifest,
    pub signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferReply {
    pub id: String,
    pub token: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionStatus {
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub direction: String,
    pub peer_name: String,
    pub title: String,
    pub kind: ContentKind,
    pub status: String,
    pub code: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub count: usize,
    pub error: Option<String>,
    pub saved_path: Option<String>,
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub id: String,
    pub direction: String,
    pub peer_name: String,
    pub title: String,
    pub kind: ContentKind,
    pub total_bytes: u64,
    pub status: String,
    pub saved_path: Option<String>,
    pub time: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub device: Device,
    pub addresses: Vec<String>,
    pub receive_dir: String,
    pub receive_dir_warning: Option<String>,
    pub peers: Vec<Peer>,
    pub transfers: Vec<Transfer>,
    pub history: Vec<History>,
    pub network_warning: Option<String>,
}

impl Manifest {
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }
    pub fn count(&self) -> usize {
        self.entries.iter().filter(|entry| !entry.directory).count()
    }
}
