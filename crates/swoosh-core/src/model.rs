use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const PROTOCOL: u32 = 1;
pub const SERVICE: &str = "_swoosh._tcp.local.";
pub const DEFAULT_PORT: u16 = 53318;
pub const TEXT_LIMIT: u64 = 1024 * 1024;
pub const ENTRY_LIMIT: usize = 10_000;
/// Both sides must confirm the short code within this window.
pub const CONFIRM_WINDOW: Duration = Duration::from_secs(60);
/// Concurrent transfers allowed per direction.
pub const MAX_ACTIVE_TRANSFERS: usize = 4;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Send,
    Receive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Connecting,
    AwaitingConfirmation,
    AwaitingSender,
    AwaitingReceiver,
    Transferring,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Rejected,
}

impl TransferStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Rejected
        )
    }
}

/// Receiver-side session state as seen by the sender while it polls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Waiting,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionStatus {
    pub status: SessionState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub direction: Direction,
    pub peer_name: String,
    pub title: String,
    pub kind: ContentKind,
    pub status: TransferStatus,
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
    pub direction: Direction,
    pub peer_name: String,
    pub title: String,
    pub kind: ContentKind,
    pub total_bytes: u64,
    pub status: TransferStatus,
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

impl Transfer {
    pub fn to_history(&self, time: u64) -> History {
        History {
            id: self.id.clone(),
            direction: self.direction,
            peer_name: self.peer_name.clone(),
            title: self.title.clone(),
            kind: self.kind.clone(),
            total_bytes: self.total_bytes,
            status: self.status,
            saved_path: self.saved_path.clone(),
            time,
        }
    }
}

impl Manifest {
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }
    pub fn count(&self) -> usize {
        self.entries.iter().filter(|entry| !entry.directory).count()
    }
}
