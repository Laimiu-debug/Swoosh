mod files;
mod identity;
pub mod model;
mod receiver;
mod sender;
mod store;
#[cfg(test)]
mod tests;
mod tls;

pub use files::{validate_manifest, validate_relative};
pub use model::{
    Device, Direction, Peer, Selection, Snapshot, Transfer, TransferStatus, DEFAULT_PORT,
    TEXT_LIMIT,
};
pub use tls::normalize_address;

use anyhow::{bail, Context, Result};
use files::SourcePlan;
use identity::Identity;
use model::{Manifest, CONFIRM_WINDOW};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

pub struct Config {
    pub data_dir: PathBuf,
    pub receive_dir: PathBuf,
    pub port: u16,
    pub name: Option<String>,
    pub discovery: bool,
    pub listen_ip: Ipv4Addr,
}

struct RuntimeState {
    peers: HashMap<String, Peer>,
    transfers: Vec<Transfer>,
    plans: HashMap<String, SourcePlan>,
    warning: Option<String>,
}

pub(crate) struct Incoming {
    manifest: Manifest,
    token: String,
    approved: bool,
    sender_confirmed: bool,
    cancelled: CancellationToken,
    completed: HashSet<usize>,
    active: HashSet<usize>,
    staging: PathBuf,
    receive_dir: PathBuf,
    text: Option<String>,
    finished: bool,
    created: std::time::Instant,
    last_activity: std::time::Instant,
}

impl Incoming {
    fn is_open(&self) -> bool {
        !self.cancelled.is_cancelled() && !self.finished
    }

    fn confirm_window_open(&self) -> bool {
        self.created.elapsed() < CONFIRM_WINDOW
    }

    fn completed_bytes(&self) -> u64 {
        self.completed
            .iter()
            .map(|&index| self.manifest.entries[index].size)
            .sum()
    }
}

pub(crate) struct Outgoing {
    confirmed: AtomicBool,
    cancelled: CancellationToken,
}

pub struct Core {
    identity: Identity,
    receive_location: std::sync::RwLock<ReceiveLocation>,
    default_receive_dir: PathBuf,
    port: u16,
    state: RwLock<RuntimeState>,
    incoming: Mutex<HashMap<String, Incoming>>,
    outgoing: Mutex<HashMap<String, Arc<Outgoing>>>,
    nonces: Mutex<HashMap<String, std::time::Instant>>,
    store: store::Store,
    mdns: Option<mdns_sd::ServiceDaemon>,
    server: axum_server::Handle,
    shutdown: CancellationToken,
}

struct ReceiveLocation {
    path: PathBuf,
    warning: Option<String>,
}

impl ReceiveLocation {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            warning: None,
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// How a transfer ended, recorded on the task and in history.
#[derive(Default)]
pub(crate) struct Outcome {
    pub error: Option<String>,
    pub saved_path: Option<String>,
    pub text: Option<String>,
}

impl Outcome {
    pub(crate) fn error(error: impl ToString) -> Self {
        Self {
            error: Some(error.to_string()),
            ..Self::default()
        }
    }
}

impl Core {
    pub async fn start(config: Config) -> Result<Arc<Self>> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        std::fs::create_dir_all(&config.data_dir)?;
        let identity = Identity::load(&config.data_dir, config.name)?;
        let store = store::Store::open(&config.data_dir.join("swoosh.sqlite3"))?;
        let custom = store.receive_dir()?;
        let receive_location = match custom.map(|path| prepare_receive_dir(&path, false)) {
            Some(Ok(path)) => ReceiveLocation::new(path),
            Some(Err(_)) => ReceiveLocation {
                path: prepare_receive_dir(&config.receive_dir, true)?,
                warning: Some(
                    "之前设置的接收文件夹不可用，暂时使用默认位置。可重新选择文件夹。".into(),
                ),
            },
            None => ReceiveLocation::new(prepare_receive_dir(&config.receive_dir, true)?),
        };
        let listener = std::net::TcpListener::bind((config.listen_ip, config.port))
            .or_else(|_| std::net::TcpListener::bind((config.listen_ip, 0)))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
            identity.cert_pem.as_bytes().to_vec(),
            identity.key_pem.as_bytes().to_vec(),
        )
        .await?;
        let mdns = if config.discovery {
            mdns_sd::ServiceDaemon::new().ok()
        } else {
            None
        };
        let core = Arc::new(Self {
            identity,
            receive_location: std::sync::RwLock::new(receive_location),
            default_receive_dir: config.receive_dir,
            port,
            store,
            mdns,
            state: RwLock::new(RuntimeState {
                peers: HashMap::new(),
                transfers: Vec::new(),
                plans: HashMap::new(),
                warning: None,
            }),
            incoming: Mutex::new(HashMap::new()),
            outgoing: Mutex::new(HashMap::new()),
            nonces: Mutex::new(HashMap::new()),
            server: axum_server::Handle::new(),
            shutdown: CancellationToken::new(),
        });
        let server_core = core.clone();
        let router = receiver::router(core.clone());
        let handle = core.server.clone();
        tokio::spawn(async move {
            if let Err(error) = axum_server::from_tcp_rustls(listener, tls)
                .handle(handle)
                .serve(router.into_make_service())
                .await
            {
                server_core.state.write().await.warning = Some(format!("接收服务停止：{error}"));
            }
        });
        if config.discovery {
            if let Err(error) = core.start_discovery().await {
                core.state.write().await.warning =
                    Some(format!("自动发现不可用，可使用手动连接：{error}"));
            }
        }
        let cleanup_core = core.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cleanup_core.shutdown.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => cleanup_core.expire_sessions().await,
                }
            }
        });
        Ok(core)
    }

    pub fn device(&self) -> &Device {
        &self.identity.device
    }
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn receive_dir(&self) -> PathBuf {
        self.receive_location.read().unwrap().path.clone()
    }

    pub fn set_receive_dir(&self, path: PathBuf) -> Result<PathBuf> {
        let path = prepare_receive_dir(&path, false)?;
        self.apply_receive_dir(path, true)
    }

    pub fn reset_receive_dir(&self) -> Result<PathBuf> {
        let path = prepare_receive_dir(&self.default_receive_dir, true)?;
        self.apply_receive_dir(path, false)
    }

    fn apply_receive_dir(&self, path: PathBuf, custom: bool) -> Result<PathBuf> {
        let mut location = self.receive_location.write().unwrap();
        self.store
            .set_receive_dir(custom.then_some(path.as_path()))?;
        *location = ReceiveLocation::new(path.clone());
        Ok(path)
    }

    pub async fn snapshot(&self) -> Result<Snapshot> {
        let state = self.state.read().await;
        let mut peers: Vec<_> = state.peers.values().cloned().collect();
        peers.sort_by(|a, b| a.device.name.cmp(&b.device.name));
        let addresses = local_addresses()
            .into_iter()
            .map(|ip| SocketAddr::new(ip, self.port).to_string())
            .collect();
        let location = self.receive_location.read().unwrap();
        Ok(Snapshot {
            device: self.device().clone(),
            addresses,
            receive_dir: location.path.to_string_lossy().into_owned(),
            receive_dir_warning: location.warning.clone(),
            peers,
            transfers: state.transfers.clone(),
            history: self.store.list()?,
            network_warning: state.warning.clone(),
        })
    }

    pub async fn select(&self, paths: Vec<PathBuf>) -> Result<Selection> {
        let plan = tokio::task::spawn_blocking(move || files::collect(paths)).await??;
        let selection = plan.selection.clone();
        let mut state = self.state.write().await;
        state.plans.clear();
        state.plans.insert(selection.id.clone(), plan);
        Ok(selection)
    }

    pub async fn clear_selection(&self) {
        self.state.write().await.plans.clear();
    }
    pub fn clear_history(&self) -> Result<()> {
        self.store.clear()
    }

    pub async fn connect(&self, input: &str) -> Result<Peer> {
        self.connect_expected(input, None, false).await
    }

    async fn connect_expected(
        &self,
        input: &str,
        expected: Option<String>,
        discovered: bool,
    ) -> Result<Peer> {
        let address = normalize_address(input).context("连接地址格式错误")?;
        let (client, observed) = tls::client(expected)?;
        let device: Device = client
            .get(format!("https://{address}/v1/info"))
            .send()
            .await
            .context("无法连接，请检查两端是否打开 Swoosh、网络隔离和防火墙")?
            .error_for_status()?
            .json()
            .await?;
        let fingerprint = observed.lock().unwrap().clone().context("未取得设备证书")?;
        if device.fingerprint != fingerprint
            || device.protocol != model::PROTOCOL
            || device.id.len() != 64
            || device.name.len() > 256
            || device.signing_key.len() != 64
            || identity::digest(hex::decode(&device.signing_key)?) != device.id
        {
            bail!("设备信息或协议不匹配");
        }
        if device.id == self.device().id {
            bail!("这是当前设备，请连接另一台设备");
        }
        let peer = Peer {
            device,
            address,
            discovered,
        };
        self.state
            .write()
            .await
            .peers
            .insert(peer.device.id.clone(), peer.clone());
        Ok(peer)
    }

    async fn start_discovery(self: &Arc<Self>) -> Result<()> {
        let mdns = self.mdns.as_ref().context("无法启动 mDNS")?;
        mdns.disable_interface(mdns_sd::IfKind::IPv6)?;
        let info = mdns_sd::ServiceInfo::new(
            model::SERVICE,
            &self.device().id[..16],
            &format!("swoosh-{}.local.", &self.device().id[..16]),
            "",
            self.port,
            &[
                ("id", self.device().id.as_str()),
                ("fingerprint", self.device().fingerprint.as_str()),
                ("version", "1"),
            ][..],
        )?
        .enable_addr_auto();
        mdns.register(info)?;
        let events = mdns.browse(model::SERVICE)?;
        let core = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = core.shutdown.cancelled() => break,
                    event = events.recv_async() => {
                        match event {
                            Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                                if info.get_property_val_str("id") == Some(core.device().id.as_str()) { continue; }
                                let expected = info.get_property_val_str("fingerprint").map(str::to_owned);
                                if expected.as_ref().is_none_or(|fp| fp.len() != 64) { continue; }
                                for ip in info.get_addresses() {
                                    let address = SocketAddr::new(*ip, info.get_port()).to_string();
                                    if core.connect_expected(&address, expected.clone(), true).await.is_ok() { break; }
                                }
                            }
                            Ok(mdns_sd::ServiceEvent::ServiceRemoved(_, fullname)) => {
                                let mut state = core.state.write().await;
                                state.peers.retain(|id, peer| !peer.discovered || !fullname.starts_with(&id[..16]));
                            }
                            Err(_) => break,
                            _ => {}
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn confirm_send(&self, id: &str) -> Result<()> {
        let outgoing = self
            .outgoing
            .lock()
            .await
            .get(id)
            .cloned()
            .context("传输已结束")?;
        outgoing.confirmed.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub async fn respond(&self, id: &str, accept: bool) -> Result<()> {
        let mut sessions = self.incoming.lock().await;
        let session = sessions.get_mut(id).context("请求已过期")?;
        if session.cancelled.is_cancelled() || !session.confirm_window_open() || session.approved {
            bail!("请求已过期或已处理");
        }
        if !accept {
            session.cancelled.cancel();
            drop(sessions);
            self.finish_task(id, TransferStatus::Rejected, Outcome::default())
                .await;
            return Ok(());
        }
        if matches!(session.manifest.kind, model::ContentKind::Files) {
            tokio::fs::create_dir(&session.staging).await?;
            for entry in &session.manifest.entries {
                let path = session.staging.join(&entry.path);
                let dir = if entry.directory {
                    path.as_path()
                } else {
                    path.parent().unwrap()
                };
                tokio::fs::create_dir_all(dir).await?;
            }
        }
        session.approved = true;
        drop(sessions);
        self.set_status(id, TransferStatus::AwaitingSender).await;
        Ok(())
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        if let Some(outgoing) = self.outgoing.lock().await.get(id).cloned() {
            outgoing.cancelled.cancel();
        }
        let staging = {
            let mut sessions = self.incoming.lock().await;
            sessions.get_mut(id).map(|session| {
                session.cancelled.cancel();
                session.staging.clone()
            })
        };
        self.finish_task(id, TransferStatus::Cancelled, Outcome::default())
            .await;
        if let Some(path) = staging {
            let _ = tokio::fs::remove_dir_all(path).await;
        }
        Ok(())
    }

    pub async fn dismiss(&self, id: &str) {
        self.state
            .write()
            .await
            .transfers
            .retain(|task| task.id != id || !task.status.is_terminal());
    }

    async fn update_task(&self, id: &str, update: impl FnOnce(&mut Transfer)) {
        if let Some(task) = self
            .state
            .write()
            .await
            .transfers
            .iter_mut()
            .find(|task| task.id == id)
        {
            if !task.status.is_terminal() {
                update(task);
            }
        }
    }

    async fn set_status(&self, id: &str, status: TransferStatus) {
        self.update_task(id, |task| task.status = status).await;
    }

    async fn set_progress(&self, id: &str, bytes: u64) {
        self.update_task(id, |task| task.transferred_bytes = bytes)
            .await;
    }

    async fn add_task(&self, task: Transfer) {
        let mut state = self.state.write().await;
        if state.transfers.len() >= 20 {
            if let Some(index) = state.transfers.iter().position(|t| t.status.is_terminal()) {
                state.transfers.remove(index);
            }
        }
        state.transfers.push(task);
    }

    async fn finish_task(&self, id: &str, status: TransferStatus, outcome: Outcome) {
        debug_assert!(status.is_terminal());
        let mut state = self.state.write().await;
        let Some(task) = state.transfers.iter_mut().find(|task| task.id == id) else {
            return;
        };
        if task.status.is_terminal() {
            return;
        }
        task.status = status;
        task.error = outcome.error;
        task.saved_path = outcome.saved_path;
        task.text = outcome.text;
        if status == TransferStatus::Completed {
            task.transferred_bytes = task.total_bytes;
        }
        let history = task.to_history(now());
        if let Err(error) = self.store.save(&history) {
            state.warning = Some(format!("历史记录保存失败：{error}"));
        }
    }

    async fn expire_sessions(&self) {
        let expired: Vec<_> = {
            let sessions = self.incoming.lock().await;
            sessions
                .iter()
                .filter(|(_, session)| {
                    let unconfirmed = !session.approved || !session.sender_confirmed;
                    session.is_open()
                        && ((unconfirmed && !session.confirm_window_open())
                            || session.last_activity.elapsed() > Duration::from_secs(120))
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in expired {
            let _ = self.cancel(&id).await;
        }
        self.nonces
            .lock()
            .await
            .retain(|_, time| time.elapsed() < Duration::from_secs(600));
        self.incoming.lock().await.retain(|_, session| {
            session.created.elapsed() < Duration::from_secs(600) || session.is_open()
        });
    }

    pub fn stop(&self) {
        self.shutdown.cancel();
        self.server.graceful_shutdown(Some(Duration::from_secs(1)));
        if let Some(mdns) = &self.mdns {
            let _ = mdns.shutdown();
        }
    }

    pub async fn received_path(&self, input: &str) -> Result<PathBuf> {
        let path = Path::new(input)
            .canonicalize()
            .context("文件已移动或删除")?;
        if path == self.receive_dir() {
            return Ok(path);
        }
        let received = |direction, status| {
            direction == Direction::Receive && status == TransferStatus::Completed
        };
        let mut roots: Vec<String> = self
            .store
            .list()?
            .into_iter()
            .filter(|item| received(item.direction, item.status))
            .filter_map(|item| item.saved_path)
            .collect();
        roots.extend(
            self.state
                .read()
                .await
                .transfers
                .iter()
                .filter(|item| received(item.direction, item.status))
                .filter_map(|item| item.saved_path.clone()),
        );
        for root in roots {
            if let Ok(root) = Path::new(&root).canonicalize() {
                if path.starts_with(root) {
                    return Ok(path);
                }
            }
        }
        bail!("只能打开当前接收文件夹或已接收的内容")
    }
}

fn prepare_receive_dir(path: &Path, create: bool) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("请选择完整的文件夹路径");
    }
    if create {
        std::fs::create_dir_all(path).context("无法创建默认接收文件夹")?;
    }
    let path = path.canonicalize().context("接收文件夹不存在或无法访问")?;
    if !path.is_dir() {
        bail!("请选择文件夹，不能选择文件");
    }
    let probe = path.join(format!(".swoosh-{}.write-check", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .context("无法写入此文件夹，请选择有写入权限的位置")?;
    use std::io::Write;
    let writable = file.write_all(b"Swoosh");
    drop(file);
    let cleanup = std::fs::remove_file(probe);
    writable.context("无法写入此文件夹，请检查权限或磁盘空间")?;
    cleanup.context("无法清理文件夹写入检查文件")?;
    Ok(path)
}

fn local_addresses() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .map(|interface| interface.ip())
        .filter(|ip| {
            ip.is_ipv4()
                && normalize_address(&SocketAddr::new(*ip, DEFAULT_PORT).to_string()).is_ok()
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}
